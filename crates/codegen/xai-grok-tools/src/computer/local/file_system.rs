use std::{
    future::Future,
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use tokio::{fs, time::sleep};

use crate::computer::types::{AsyncFileSystem, ComputerError};

/// Creates a local FS access which allows writing and reading from the local files
pub struct LocalFs;

// Keep the window short: these retries absorb brief Windows editor/indexer/AV
// races without hiding persistent locks, ACL failures, or sandbox denials.
const WRITE_RETRY_DELAYS: &[Duration] = &[
    Duration::from_millis(25),
    Duration::from_millis(50),
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(400),
];
#[cfg(any(windows, test))]
const WINDOWS_ERROR_SHARING_VIOLATION: i32 = 32;
#[cfg(any(windows, test))]
const WINDOWS_ERROR_LOCK_VIOLATION: i32 = 33;

/// Check if an IO error is a permission denial (EACCES or EPERM),
/// which indicates a sandbox violation.
fn is_permission_error(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::PermissionDenied)
}

#[cfg(any(windows, test))]
fn is_windows_transient_write_lock_raw_os_error(raw_os_error: Option<i32>) -> bool {
    matches!(
        raw_os_error,
        Some(WINDOWS_ERROR_SHARING_VIOLATION | WINDOWS_ERROR_LOCK_VIOLATION)
    )
}

fn is_transient_write_lock_error(e: &io::Error) -> bool {
    #[cfg(windows)]
    {
        is_windows_transient_write_lock_raw_os_error(e.raw_os_error())
    }

    #[cfg(not(windows))]
    {
        let _ = e;
        false
    }
}

#[cfg(test)]
fn is_test_transient_write_lock_error(e: &io::Error) -> bool {
    is_windows_transient_write_lock_raw_os_error(e.raw_os_error())
}

/// Write `data` to `path` by filling a sibling temp file and renaming it into place.
/// A failure while that temp is written removes the temp and leaves `path` unchanged,
/// so a torn write cannot replace the previous bytes with an empty file.
pub async fn write_replacing_previous(path: &Path, data: &[u8]) -> io::Result<()> {
    let dest = followed_write_target(path).await?;
    if let Ok(meta) = fs::metadata(&dest).await
        && meta.is_dir()
    {
        return Err(io::Error::new(
            io::ErrorKind::IsADirectory,
            "refusing to replace a directory",
        ));
    }
    let tmp = sibling_temp_path(&dest);
    if let Err(error) = fs::write(&tmp, data).await {
        let _ = fs::remove_file(&tmp).await;
        return Err(error);
    }
    if let Err(error) = finish_replace(&tmp, &dest).await {
        let _ = fs::remove_file(&tmp).await;
        return Err(error);
    }
    Ok(())
}

/// `File::create` follows a symlink. A dangling link is an error and stays a link.
async fn followed_write_target(path: &Path) -> io::Result<PathBuf> {
    match fs::symlink_metadata(path).await {
        Ok(meta) if meta.file_type().is_symlink() => fs::canonicalize(path).await,
        Ok(_) => Ok(path.to_path_buf()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error),
    }
}

fn sibling_temp_path(path: &Path) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nonce = SEQ.fetch_add(1, Ordering::Relaxed);
    let file_name = format!(".{}.{nonce}.grok-write.tmp", std::process::id());
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.join(file_name),
        _ => PathBuf::from(file_name),
    }
}

async fn finish_replace(tmp: &Path, dest: &Path) -> io::Result<()> {
    copy_existing_mode(tmp, dest).await?;
    fs::rename(tmp, dest).await
}

async fn copy_existing_mode(tmp: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        match fs::symlink_metadata(dest).await {
            Ok(meta) if meta.is_file() => {
                fs::set_permissions(tmp, meta.permissions()).await?;
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (tmp, dest);
    }
    Ok(())
}

async fn write_file_with_transient_lock_retries(path: &Path, data: &[u8]) -> io::Result<()> {
    write_file_with_retry_hooks(
        || write_replacing_previous(path, data),
        |delay| sleep(delay),
        |retry_count| {
            tracing::debug!(
                path = %path.display(),
                retry_count,
                "file write succeeded after transient lock retries"
            );
        },
        |error, retry_count, delay| {
            tracing::debug!(
                path = %path.display(),
                error = %error,
                retry_count,
                delay_ms = delay.as_millis(),
                "file write hit transient lock; retrying"
            );
        },
        |error, retry_count| {
            tracing::debug!(
                path = %path.display(),
                error = %error,
                retry_count,
                "file write exhausted transient lock retries"
            );
        },
        is_transient_write_lock_error,
    )
    .await
}

async fn write_file_with_retry_hooks<W, WFut, S, SFut, Success, Retry, Exhausted, IsRetryable>(
    mut write: W,
    mut sleep_for: S,
    mut on_retry_success: Success,
    mut on_retry: Retry,
    mut on_exhausted: Exhausted,
    is_retryable: IsRetryable,
) -> io::Result<()>
where
    W: FnMut() -> WFut,
    WFut: Future<Output = io::Result<()>>,
    S: FnMut(Duration) -> SFut,
    SFut: Future<Output = ()>,
    Success: FnMut(usize),
    Retry: FnMut(&io::Error, usize, Duration),
    Exhausted: FnMut(&io::Error, usize),
    IsRetryable: Fn(&io::Error) -> bool,
{
    let mut retry_count = 0usize;

    loop {
        match write().await {
            Ok(()) => {
                if retry_count > 0 {
                    on_retry_success(retry_count);
                }
                return Ok(());
            }
            Err(e) if is_retryable(&e) => {
                if retry_count >= WRITE_RETRY_DELAYS.len() {
                    on_exhausted(&e, retry_count);
                    return Err(e);
                }
                let Some(&delay) = WRITE_RETRY_DELAYS.get(retry_count) else {
                    on_exhausted(&e, retry_count);
                    return Err(e);
                };
                retry_count += 1;
                on_retry(&e, retry_count, delay);
                sleep_for(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

#[async_trait::async_trait]
impl AsyncFileSystem for LocalFs {
    #[tracing::instrument(name = "fs.read_file", skip_all)]
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, ComputerError> {
        match crate::util::file_reader::read_file(
            path,
            crate::util::file_reader::FileReadOptions::default(),
        )
        .await
        {
            Ok(data) => Ok(data),
            Err(e) => {
                if is_permission_error(&e) {
                    xai_grok_sandbox::log_violation(&path.display().to_string(), "read");
                }
                Err(e.into())
            }
        }
    }

    fn supports_bounded_read(&self) -> bool {
        true
    }

    #[tracing::instrument(name = "fs.read_file_bounded", skip_all)]
    async fn read_file_bounded(
        &self,
        path: &Path,
        max_bytes: usize,
    ) -> Result<Vec<u8>, ComputerError> {
        use crate::util::file_reader::{FileReadMode, FileReadOptions, read_file};

        read_file(
            path,
            FileReadOptions {
                mode: FileReadMode::BoundedComplete { max_bytes },
                require_regular_file: true,
            },
        )
        .await
        .map_err(|error| {
            if is_permission_error(&error) {
                xai_grok_sandbox::log_violation(&path.display().to_string(), "read");
            }
            ComputerError::from(error)
        })
    }

    #[tracing::instrument(name = "fs.write_file", skip_all)]
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), ComputerError> {
        // implicitly creates the missing directories if any
        if let Some(dir) = path.parent()
            && let Err(e) = fs::create_dir_all(dir).await
        {
            if is_permission_error(&e) {
                xai_grok_sandbox::log_violation(&dir.display().to_string(), "mkdir");
            }
            return Err(e.into());
        }
        if let Err(e) = write_file_with_transient_lock_retries(path, data).await {
            if is_permission_error(&e) {
                xai_grok_sandbox::log_violation(&path.display().to_string(), "write");
            }
            return Err(e.into());
        }
        Ok(())
    }

    #[tracing::instrument(name = "fs.file_exists", skip_all)]
    async fn file_exists(&self, path: &Path) -> Result<bool, ComputerError> {
        match fs::metadata(path).await {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    #[tracing::instrument(name = "fs.delete_file", skip_all)]
    async fn delete_file(&self, path: &Path) -> Result<(), ComputerError> {
        if let Err(e) = fs::remove_file(path).await {
            if is_permission_error(&e) {
                xai_grok_sandbox::log_violation(&path.display().to_string(), "delete");
            }
            return Err(e.into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn classifies_windows_transient_write_lock_errors() {
        assert!(is_windows_transient_write_lock_raw_os_error(Some(32)));
        assert!(is_windows_transient_write_lock_raw_os_error(Some(33)));
        assert!(!is_windows_transient_write_lock_raw_os_error(Some(5)));
        assert!(!is_windows_transient_write_lock_raw_os_error(None));
    }

    #[cfg(windows)]
    #[test]
    fn classifies_windows_transient_write_lock_io_errors() {
        assert!(is_transient_write_lock_error(
            &io::Error::from_raw_os_error(WINDOWS_ERROR_SHARING_VIOLATION,)
        ));
        assert!(is_transient_write_lock_error(
            &io::Error::from_raw_os_error(WINDOWS_ERROR_LOCK_VIOLATION,)
        ));
        assert!(!is_transient_write_lock_error(
            &io::Error::from_raw_os_error(5)
        ));
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_does_not_retry_windows_raw_error_numbers() {
        assert!(!is_transient_write_lock_error(
            &io::Error::from_raw_os_error(WINDOWS_ERROR_SHARING_VIOLATION,)
        ));
    }

    #[tokio::test]
    async fn first_attempt_success_does_not_fire_retry_callbacks() {
        let result = write_file_with_retry_hooks(
            || async { Ok(()) },
            |_| async { panic!("should not sleep") },
            |_| panic!("should not fire on_retry_success"),
            |_, _, _| panic!("should not fire on_retry"),
            |_, _| panic!("should not fire on_exhausted"),
            is_test_transient_write_lock_error,
        )
        .await;
        result.unwrap();
    }

    #[tokio::test]
    async fn transient_lock_errors_are_retried_until_success() {
        let attempts = Rc::new(Cell::new(0usize));
        let sleeps = Rc::new(Cell::new(0usize));
        let retry_success_count = Rc::new(Cell::new(0usize));
        let retry_log_count = Rc::new(Cell::new(0usize));

        let result = write_file_with_retry_hooks(
            {
                let attempts = Rc::clone(&attempts);
                move || {
                    let attempts = Rc::clone(&attempts);
                    async move {
                        let next = attempts.get() + 1;
                        attempts.set(next);
                        if next <= 2 {
                            Err(io::Error::from_raw_os_error(
                                WINDOWS_ERROR_SHARING_VIOLATION,
                            ))
                        } else {
                            Ok(())
                        }
                    }
                }
            },
            {
                let sleeps = Rc::clone(&sleeps);
                move |_| {
                    let sleeps = Rc::clone(&sleeps);
                    async move {
                        sleeps.set(sleeps.get() + 1);
                    }
                }
            },
            |count| retry_success_count.set(count),
            |_, _, _| retry_log_count.set(retry_log_count.get() + 1),
            |_, _| panic!("retry budget should not be exhausted"),
            is_test_transient_write_lock_error,
        )
        .await;

        result.unwrap();
        assert_eq!(attempts.get(), 3);
        assert_eq!(sleeps.get(), 2);
        assert_eq!(retry_log_count.get(), 2);
        assert_eq!(retry_success_count.get(), 2);
    }

    #[tokio::test]
    async fn non_transient_errors_are_not_retried() {
        let attempts = Rc::new(Cell::new(0usize));
        let result = write_file_with_retry_hooks(
            {
                let attempts = Rc::clone(&attempts);
                move || {
                    let attempts = Rc::clone(&attempts);
                    async move {
                        attempts.set(attempts.get() + 1);
                        Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
                    }
                }
            },
            |_| async {},
            |_| panic!("write did not succeed"),
            |_, _, _| panic!("non-transient errors must not be retried"),
            |_, _| panic!("non-transient errors must not exhaust retry budget"),
            is_test_transient_write_lock_error,
        )
        .await;

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
        assert_eq!(attempts.get(), 1);
    }

    #[tokio::test]
    async fn persistent_transient_lock_exhausts_retry_budget() {
        let attempts = Rc::new(Cell::new(0usize));
        let sleeps = Rc::new(Cell::new(0usize));
        let exhausted_count = Rc::new(Cell::new(0usize));

        let result = write_file_with_retry_hooks(
            {
                let attempts = Rc::clone(&attempts);
                move || {
                    let attempts = Rc::clone(&attempts);
                    async move {
                        attempts.set(attempts.get() + 1);
                        Err(io::Error::from_raw_os_error(WINDOWS_ERROR_LOCK_VIOLATION))
                    }
                }
            },
            {
                let sleeps = Rc::clone(&sleeps);
                move |_| {
                    let sleeps = Rc::clone(&sleeps);
                    async move {
                        sleeps.set(sleeps.get() + 1);
                    }
                }
            },
            |_| panic!("write did not succeed"),
            |_, _, _| {},
            |_, count| exhausted_count.set(count),
            is_test_transient_write_lock_error,
        )
        .await;

        assert_eq!(
            result.unwrap_err().raw_os_error(),
            Some(WINDOWS_ERROR_LOCK_VIOLATION)
        );
        assert_eq!(attempts.get(), WRITE_RETRY_DELAYS.len() + 1);
        assert_eq!(sleeps.get(), WRITE_RETRY_DELAYS.len());
        assert_eq!(exhausted_count.get(), WRITE_RETRY_DELAYS.len());
    }

    fn grok_write_temps(dir: &std::path::Path) -> Vec<std::ffi::OsString> {
        std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().contains("grok-write"))
            .collect()
    }

    #[tokio::test]
    async fn preserving_file_write_replaces_contents_when_the_write_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        LocalFs.write_file(&path, b"previous").await.unwrap();
        LocalFs
            .write_file(&path, b"next\0byte")
            .await
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"next\0byte");
        assert!(grok_write_temps(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn preserving_file_write_creates_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("new.txt");
        LocalFs.write_file(&path, b"created").await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"created");
        assert!(grok_write_temps(&dir.path().join("sub")).is_empty());
    }

    #[tokio::test]
    async fn preserving_file_write_refuses_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("folder");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("inside.txt"), b"keep").unwrap();
        assert!(LocalFs.write_file(&target, b"nope").await.is_err());
        assert!(target.is_dir());
        assert_eq!(std::fs::read(target.join("inside.txt")).unwrap(), b"keep");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn preserving_file_write_keeps_the_existing_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("script.sh");
        LocalFs.write_file(&path, b"old").await.unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        LocalFs.write_file(&path, b"new").await.unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn preserving_file_write_follows_a_symlink_and_keeps_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        std::fs::write(&target, b"old").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        LocalFs.write_file(&link, b"new").await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(grok_write_temps(dir.path()).is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn preserving_file_write_leaves_a_dangling_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(dir.path().join("missing.txt"), &link).unwrap();
        assert!(LocalFs.write_file(&link, b"new").await.is_err());
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(!dir.path().join("missing.txt").exists());
    }

    /// The directory cannot accept a new temp name, while the file itself stays writable.
    /// An in-place truncate would replace `previous`. This write fails and leaves it.
    #[cfg(unix)]
    #[tokio::test]
    async fn preserving_file_write_leaves_previous_contents_when_the_write_fails() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        LocalFs.write_file(&path, b"previous").await.unwrap();

        struct RestoreMode {
            path: std::path::PathBuf,
            mode: u32,
        }
        impl Drop for RestoreMode {
            fn drop(&mut self) {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &self.path,
                    std::fs::Permissions::from_mode(self.mode),
                );
            }
        }
        let original = std::fs::metadata(dir.path()).unwrap().permissions().mode();
        let _restore = RestoreMode {
            path: dir.path().to_path_buf(),
            mode: original,
        };
        let mut locked = std::fs::metadata(dir.path()).unwrap().permissions();
        locked.set_mode(0o555);
        std::fs::set_permissions(dir.path(), locked).unwrap();

        assert!(
            LocalFs
                .write_file(&path, b"replacement that must not land")
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        assert!(grok_write_temps(dir.path()).is_empty());
    }
}
