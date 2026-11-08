//! Copy-on-Write file cloning using the reflink-copy crate.
//!
//! Uses reflink (CoW) when supported by the filesystem, with automatic
//! fallback to regular copy.

use std::path::Path;

use anyhow::Result;

/// Reflink (APFS/Btrfs/XFS) when the filesystem supports it; otherwise a regular copy.
/// A mode that cannot be applied is an error: git metadata copies must keep the source mode.
pub(crate) fn clone_file(src: &Path, dest: &Path) -> Result<()> {
    match clone_worktree_file(src, dest)? {
        WorktreeFileCopy::Copied => Ok(()),
        WorktreeFileCopy::ModeLost(err) => Err(err.into()),
    }
}

/// How a worktree file copy finished. The bytes are in `dest` either way.
pub(crate) enum WorktreeFileCopy {
    Copied,
    /// The file is in place, but its mode does not match the source.
    ModeLost(std::io::Error),
}

/// Copy one worktree file. A permission failure keeps the bytes and reports [`WorktreeFileCopy::ModeLost`].
pub(crate) fn clone_worktree_file(src: &Path, dest: &Path) -> Result<WorktreeFileCopy> {
    reflink_copy::reflink_or_copy(src, dest)?;
    match apply_source_mode(src, dest) {
        Ok(()) => Ok(WorktreeFileCopy::Copied),
        Err(err) => Ok(WorktreeFileCopy::ModeLost(err)),
    }
}

/// Set `dest`'s mode from `src`. Reflink (`FICLONE`) creates `dest` with the umask mode, so the
/// executable bit would otherwise be lost.
pub(crate) fn apply_source_mode(src: &Path, dest: &Path) -> std::io::Result<()> {
    let perms = std::fs::metadata(src)?.permissions();
    std::fs::set_permissions(dest, perms)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let want = std::fs::metadata(src)?.permissions().mode() & 0o777;
        let got = std::fs::metadata(dest)?.permissions().mode() & 0o777;
        if want != got {
            return Err(std::io::Error::other(format!(
                "source mode is {want:o}, copy kept {got:o}"
            )));
        }
    }
    Ok(())
}

/// Point `dst` at `target`, replacing any existing entry. `symlink()` will not
/// overwrite, so remove `dst` first (a missing path is not an error).
pub(crate) fn replace_symlink(target: &Path, dst: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(dst);
    symlink_to(target, dst)
}

#[cfg(unix)]
fn symlink_to(target: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, dst)
}

#[cfg(windows)]
fn symlink_to(target: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_clone_file_with_fallback() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("source.txt");
        let dest = temp.path().join("dest.txt");

        std::fs::write(&src, "hello world").unwrap();

        // Should work either via CoW or fallback
        clone_file(&src, &dest).unwrap();

        assert!(dest.exists());
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "hello world");
    }

    #[test]
    fn test_clone_file_binary() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("source.bin");
        let dest = temp.path().join("dest.bin");

        let data: Vec<u8> = (0..=255).collect();
        std::fs::write(&src, &data).unwrap();

        clone_file(&src, &dest).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), data);
    }

    #[test]
    fn test_clone_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let src = temp.path().join("script.sh");
        let dest = temp.path().join("script_copy.sh");

        std::fs::write(&src, "#!/bin/bash\necho hello").unwrap();

        // Make executable
        let mut perms = std::fs::metadata(&src).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&src, perms).unwrap();

        clone_file(&src, &dest).unwrap();

        let dest_perms = std::fs::metadata(&dest).unwrap().permissions();
        assert_eq!(dest_perms.mode() & 0o777, 0o755);
    }

    /// `/proc` rejects chmod. The mode helper must surface that as a lost mode, not a successful copy.
    #[cfg(unix)]
    #[test]
    fn apply_source_mode_reports_a_filesystem_that_refuses_chmod() {
        use std::os::unix::fs::PermissionsExt;
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("script.sh");
        std::fs::write(&src, "#!/bin/sh\n").unwrap();
        let mut perms = std::fs::metadata(&src).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&src, perms).unwrap();

        let err = apply_source_mode(&src, Path::new("/proc/self/status")).unwrap_err();
        let listed = format!("mode change lost for script.sh: {err}");
        assert!(
            listed.starts_with("mode change lost for script.sh:"),
            "{listed}"
        );
        assert!(
            listed.contains("Operation not permitted") || listed.contains("os error 1"),
            "{listed}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_replace_symlink_overwrites_and_allows_dangling() {
        let temp = TempDir::new().unwrap();
        let dst = temp.path().join("link");

        std::fs::write(&dst, "stale").unwrap();
        // Target is intentionally dangling; it must still be created.
        replace_symlink(Path::new("does-not-exist"), &dst).unwrap();

        let meta = std::fs::symlink_metadata(&dst).unwrap();
        assert!(meta.file_type().is_symlink(), "dst must be a symlink");
        assert_eq!(
            std::fs::read_link(&dst).unwrap(),
            Path::new("does-not-exist")
        );
    }
}
