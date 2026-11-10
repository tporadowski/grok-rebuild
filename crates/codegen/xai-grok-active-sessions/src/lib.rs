//! Tracks open TUI sessions in `~/.grok/active_sessions.json`. A clean exit removes the entry,
//! a crash leaves it behind, and the next [`register`] prunes entries whose PID is dead.
//! A file that does not parse is backed up. Rows that still parse are kept, and registration continues.

#![deny(clippy::indexing_slicing)]

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use agent_client_protocol as acp;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveSession {
    pub session_id: acp::SessionId,
    pub pid: u32,
    pub cwd: String,
    pub opened_at: DateTime<Utc>,
}

/// What [`register`] did with a registry file that did not parse as a session list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryRecovery {
    /// Copy of the corrupt bytes, when the copy succeeded.
    pub backup_path: Option<std::path::PathBuf>,
    /// Session rows that still parsed.
    pub salvaged: usize,
    /// Sentence for the user. Also written to the tracing log.
    pub warning: String,
}

const DATA_FILENAME: &str = "active_sessions.json";
const LOCK_FILENAME: &str = "active_sessions.lock";
const TMP_FILENAME: &str = "active_sessions.json.tmp";
const BACKUP_FILENAME: &str = "active_sessions.json.bak";

/// On an NFS home `flock` is a network-lock-manager lock with no lease: a holder killed at the
/// wrong moment strands it forever, so never wait unbounded. A live holder only does one small
/// read-modify-write, so anything held this long is stranded.
const LOCK_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(2);
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Register a session as active (idempotent by session_id) and prune entries whose PID is dead.
/// Fails with [`io::ErrorKind::TimedOut`] if the lock is still held after `LOCK_ACQUIRE_TIMEOUT`.
/// `Ok(Some(_))` means the registry file was corrupt: a backup was kept when possible, readable
/// rows were kept, and this session was still registered.
pub fn register(session: ActiveSession) -> io::Result<Option<RegistryRecovery>> {
    register_in(&xai_grok_config::grok_home(), session)
}

/// Non-blocking unregister for signal handlers.
/// Returns `Ok(false)` on lock contention; the orphan is pruned by the next `register`.
pub fn try_unregister(session_id: &acp::SessionId) -> io::Result<bool> {
    try_unregister_in(&xai_grok_config::grok_home(), session_id)
}

pub fn register_in(root: &Path, session: ActiveSession) -> io::Result<Option<RegistryRecovery>> {
    let (_, recovery) = with_locked_state(root, LOCK_ACQUIRE_TIMEOUT, |sessions| {
        sessions.retain(|s| s.session_id != session.session_id && is_pid_alive(s.pid));
        sessions.push(session);
    })?;
    Ok(recovery)
}

pub fn try_unregister_in(root: &Path, session_id: &acp::SessionId) -> io::Result<bool> {
    let outcome = with_locked_state(root, Duration::ZERO, |sessions| {
        sessions.retain(|s| s.session_id != *session_id);
    });
    match outcome {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::TimedOut => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn list_in(root: &Path) -> io::Result<Vec<ActiveSession>> {
    let (sessions, _) = read_sessions(&root.join(DATA_FILENAME))?;
    Ok(sessions)
}

fn with_locked_state<F, R>(
    root: &Path,
    timeout: Duration,
    mutate: F,
) -> io::Result<(R, Option<RegistryRecovery>)>
where
    F: FnOnce(&mut Vec<ActiveSession>) -> R,
{
    let lock_path = root.join(LOCK_FILENAME);
    let data_path = root.join(DATA_FILENAME);
    let tmp_path = root.join(TMP_FILENAME);

    fs::create_dir_all(root)?;
    // Closing the file releases the lock, so every return path unlocks by dropping it.
    let lock_file = open_lock_file(&lock_path)?;
    let deadline = Instant::now() + timeout;
    loop {
        match lock_file.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(e)) => return Err(e),
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("active-session registry lock still held after {timeout:?}"),
            ));
        }
        std::thread::sleep(LOCK_POLL_INTERVAL.min(remaining));
    }

    let (mut sessions, recovery) = read_for_update(&data_path)?;
    let result = mutate(&mut sessions);
    write_data_file_atomic(&tmp_path, &data_path, &sessions)?;
    Ok((result, recovery))
}

fn open_lock_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

fn read_sessions(path: &Path) -> io::Result<(Vec<ActiveSession>, bool)> {
    match fs::read(path) {
        Ok(bytes) if bytes_are_blank(&bytes) => Ok((Vec::new(), false)),
        Ok(bytes) => Ok(parse_registry(&bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok((Vec::new(), false)),
        Err(e) => Err(e),
    }
}

/// Parse for a rewrite. A corrupt file is copied aside before the caller replaces it.
fn read_for_update(path: &Path) -> io::Result<(Vec<ActiveSession>, Option<RegistryRecovery>)> {
    let (sessions, corrupt) = read_sessions(path)?;
    if !corrupt {
        return Ok((sessions, None));
    }
    let original = fs::read(path)?;
    let (backup_path, backup_error) = match unused_backup_path(path) {
        Some(backup) => match fs::write(&backup, &original) {
            Ok(()) => (Some(backup), None),
            Err(error) => (None, Some(error.to_string())),
        },
        None => (None, Some("too many backups already exist".to_string())),
    };
    let recovery = RegistryRecovery {
        salvaged: sessions.len(),
        warning: recovery_warning(
            backup_path.as_deref(),
            backup_error.as_deref(),
            sessions.len(),
        ),
        backup_path,
    };
    tracing::warn!("{}", recovery.warning);
    Ok((sessions, Some(recovery)))
}

fn bytes_are_blank(bytes: &[u8]) -> bool {
    bytes.iter().all(u8::is_ascii_whitespace)
}

fn parse_registry(bytes: &[u8]) -> (Vec<ActiveSession>, bool) {
    match serde_json::from_slice::<Vec<ActiveSession>>(bytes) {
        Ok(sessions) => (sessions, false),
        Err(_) => (salvage_sessions(bytes), true),
    }
}

fn salvage_sessions(bytes: &[u8]) -> Vec<ActiveSession> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Vec::new();
    };
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        return sessions_from_value(&value);
    }
    sessions_from_object_spans(text)
}

fn sessions_from_value(value: &serde_json::Value) -> Vec<ActiveSession> {
    match value {
        serde_json::Value::Array(items) => items.iter().filter_map(session_from_value).collect(),
        other => session_from_value(other).into_iter().collect(),
    }
}

fn session_from_value(value: &serde_json::Value) -> Option<ActiveSession> {
    serde_json::from_value(value.clone()).ok()
}

/// Pull every complete `{...}` object that is still a session. A truncated tail is left out.
fn sessions_from_object_spans(text: &str) -> Vec<ActiveSession> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    let mut in_string = false;
    let mut escape = false;
    while index < bytes.len() {
        let Some(byte) = bytes.get(index).copied() else {
            break;
        };
        if in_string {
            if escape {
                escape = false;
            } else if byte == b'\\' {
                escape = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            index += 1;
            continue;
        }
        if byte == b'{'
            && let Some(end) = matching_brace_end(text, index)
            && let Some(slice) = text.get(index..=end)
            && let Ok(session) = serde_json::from_str::<ActiveSession>(slice)
        {
            out.push(session);
            index = end.saturating_add(1);
            continue;
        }
        index += 1;
    }
    out
}

fn matching_brace_end(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (idx, byte) in bytes.iter().enumerate().skip(open) {
        if in_string {
            if escape {
                escape = false;
            } else if *byte == b'\\' {
                escape = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn unused_backup_path(data_path: &Path) -> Option<std::path::PathBuf> {
    let dir = data_path.parent().unwrap_or_else(|| Path::new("."));
    let primary = dir.join(BACKUP_FILENAME);
    if !primary.exists() {
        return Some(primary);
    }
    let mut n = 2u32;
    while n < 1000 {
        let candidate = dir.join(format!("active_sessions.json.{n}.bak"));
        if !candidate.exists() {
            return Some(candidate);
        }
        n += 1;
    }
    None
}

fn recovery_warning(backup: Option<&Path>, backup_error: Option<&str>, salvaged: usize) -> String {
    let rows = match salvaged {
        1 => "1 session row".to_string(),
        n => format!("{n} session rows"),
    };
    let backup_sentence = match (backup, backup_error) {
        (Some(path), _) => format!("Kept a backup at {}.", path.display()),
        (None, Some(err)) => format!("Could not keep a backup ({err})."),
        (None, None) => "Could not keep a backup.".to_string(),
    };
    format!(
        "active_sessions.json is corrupt. {backup_sentence} Salvaged {rows}. New sessions can still register."
    )
}

fn write_data_file_atomic(
    tmp_path: &Path,
    data_path: &Path,
    sessions: &[ActiveSession],
) -> io::Result<()> {
    let json = serde_json::to_string_pretty(sessions)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(tmp_path, json.as_bytes())?;
    fs::rename(tmp_path, data_path).inspect_err(|_| {
        let _ = fs::remove_file(tmp_path);
    })
}

fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let pid_i = match i32::try_from(pid) {
            Ok(p) if p > 0 => p,
            _ => return false,
        };
        let ret = unsafe { libc::kill(pid_i as libc::pid_t, 0) };
        if ret == 0 {
            return true;
        }
        io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) };
        match handle {
            Ok(h) => {
                let _ = unsafe { CloseHandle(h) };
                true
            }
            Err(_) => false,
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        // Conservative: assume alive if we can't check.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_session(id: &str, pid: u32) -> ActiveSession {
        ActiveSession {
            session_id: acp::SessionId::new(id),
            pid,
            cwd: "/tmp/test".into(),
            opened_at: Utc::now(),
        }
    }

    #[test]
    fn register_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let s = make_session("s1", std::process::id());
        register_in(dir.path(), s.clone()).unwrap();
        register_in(dir.path(), s).unwrap();
        assert_eq!(1, list_in(dir.path()).unwrap().len());
    }

    #[test]
    fn concurrent_registers_no_corruption() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        std::thread::scope(|s| {
            for i in 0..10 {
                let p = path.clone();
                s.spawn(move || {
                    register_in(&p, make_session(&format!("s{i}"), std::process::id())).unwrap()
                });
            }
        });
        assert_eq!(10, list_in(dir.path()).unwrap().len());
    }

    #[test]
    fn try_unregister_skips_if_locked() {
        let dir = TempDir::new().unwrap();
        let s = make_session("s1", std::process::id());
        register_in(dir.path(), s.clone()).unwrap();

        let lock_file = open_lock_file(&dir.path().join(LOCK_FILENAME)).unwrap();
        lock_file.lock().unwrap();
        assert!(!try_unregister_in(dir.path(), &s.session_id).unwrap());
        lock_file.unlock().unwrap();
        assert_eq!(1, list_in(dir.path()).unwrap().len());
    }

    #[test]
    fn locked_update_times_out_when_lock_stays_held() {
        let dir = TempDir::new().unwrap();
        register_in(dir.path(), make_session("s1", std::process::id())).unwrap();
        let holder = open_lock_file(&dir.path().join(LOCK_FILENAME)).unwrap();
        holder.lock().unwrap();

        let outcome = with_locked_state(dir.path(), Duration::from_millis(50), Vec::clear);

        assert_eq!(io::ErrorKind::TimedOut, outcome.unwrap_err().kind());
        assert_eq!(1, list_in(dir.path()).unwrap().len());
    }

    #[test]
    fn locked_update_retries_until_holder_releases() {
        let dir = TempDir::new().unwrap();
        let holder = open_lock_file(&dir.path().join(LOCK_FILENAME)).unwrap();
        holder.lock().unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            holder.unlock().unwrap();
        });

        with_locked_state(dir.path(), LOCK_ACQUIRE_TIMEOUT, |sessions| {
            sessions.push(make_session("s1", std::process::id()));
        })
        .unwrap();
        release.join().unwrap();
        assert_eq!(1, list_in(dir.path()).unwrap().len());
    }

    #[test]
    fn corrupt_file_recovers() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(DATA_FILENAME), "garbage{{{").unwrap();
        assert!(list_in(dir.path()).unwrap().is_empty());
        register_in(dir.path(), make_session("s1", std::process::id())).unwrap();
        assert_eq!(1, list_in(dir.path()).unwrap().len());
    }

    /// A corrupt registry is backed up, readable rows are kept, and the new session still registers.
    #[test]
    fn corrupt_registry_keeps_a_backup_salvages_rows_and_still_registers() {
        let dir = TempDir::new().unwrap();
        let good = make_session("keep", std::process::id());
        let good_json = serde_json::to_string(&good).unwrap();
        let raw = format!("[{good_json},{{\"pid\":\"nope\"}}]");
        fs::write(dir.path().join(DATA_FILENAME), &raw).unwrap();
        let listed = list_in(dir.path()).unwrap();
        assert_eq!(
            vec![good.session_id.clone()],
            listed
                .iter()
                .map(|session| session.session_id.clone())
                .collect::<Vec<_>>()
        );

        let recovery = register_in(dir.path(), make_session("fresh", std::process::id()))
            .unwrap()
            .expect("corrupt registry warns");
        let backup = recovery.backup_path.expect("backup path");
        assert_eq!(raw.as_bytes(), fs::read(&backup).unwrap().as_slice());
        assert_eq!(1, recovery.salvaged);
        assert!(
            recovery.warning.contains("active_sessions.json is corrupt"),
            "{}",
            recovery.warning
        );
        assert!(
            recovery.warning.contains(&backup.display().to_string()),
            "{}",
            recovery.warning
        );
        assert!(
            recovery.warning.contains("Salvaged 1 session row"),
            "{}",
            recovery.warning
        );
        assert!(
            recovery.warning.contains("New sessions can still register"),
            "{}",
            recovery.warning
        );
        let ids: Vec<_> = list_in(dir.path())
            .unwrap()
            .into_iter()
            .map(|session| session.session_id)
            .collect();
        assert_eq!(
            vec![acp::SessionId::new("keep"), acp::SessionId::new("fresh")],
            ids
        );
        assert!(
            register_in(dir.path(), make_session("again", std::process::id()))
                .unwrap()
                .is_none(),
            "a repaired registry does not warn again"
        );

        let truncated = TempDir::new().unwrap();
        let cut = format!("[{good_json},{{\"session_id\":\"cut\"");
        fs::write(truncated.path().join(DATA_FILENAME), &cut).unwrap();
        let recovery = register_in(truncated.path(), make_session("fresh", std::process::id()))
            .unwrap()
            .expect("truncated registry warns");
        assert_eq!(1, recovery.salvaged);
        assert_eq!(
            cut.as_bytes(),
            fs::read(recovery.backup_path.expect("truncated backup"))
                .unwrap()
                .as_slice()
        );
        let ids: Vec<_> = list_in(truncated.path())
            .unwrap()
            .into_iter()
            .map(|session| session.session_id)
            .collect();
        assert_eq!(
            vec![acp::SessionId::new("keep"), acp::SessionId::new("fresh")],
            ids
        );

        let garbage = TempDir::new().unwrap();
        fs::write(garbage.path().join(DATA_FILENAME), b"garbage{{{").unwrap();
        let recovery = register_in(garbage.path(), make_session("fresh", std::process::id()))
            .unwrap()
            .expect("unreadable json warns");
        assert_eq!(0, recovery.salvaged);
        assert!(
            recovery.warning.contains("Salvaged 0 session rows"),
            "{}",
            recovery.warning
        );
        assert_eq!(
            b"garbage{{{".as_slice(),
            fs::read(recovery.backup_path.expect("garbage backup"))
                .unwrap()
                .as_slice()
        );
        assert_eq!(1, list_in(garbage.path()).unwrap().len());

        let blank = TempDir::new().unwrap();
        fs::write(blank.path().join(DATA_FILENAME), b" \n").unwrap();
        assert!(
            register_in(blank.path(), make_session("fresh", std::process::id()))
                .unwrap()
                .is_none()
        );
        assert!(!blank.path().join(BACKUP_FILENAME).exists());
    }
}
