//! Exclusive read-modify-write for one session's `usage.json`.
//!
//! Two windows persist turns for the same session through separate actors.
//! A turn's delta is process-local, and the file is shared.
//! Writing back a stale read drops the other window's turns.
//! The sidecar `usage.json.lock` is never renamed, so one lock covers the read, the merge, and the replace.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

use fs2::FileExt;

use crate::session::usage_file::{SessionUsageFile, UsageSummary};

pub(crate) struct UsageTurnRequest {
    pub session_id: String,
    pub turn_number: u32,
    pub ended_at: String,
    pub live: UsageSummary,
    pub prev_live: Option<UsageSummary>,
    pub last_incoming_turn: Option<u32>,
    pub last_written_turn: Option<u32>,
}

pub(crate) struct UsageTurnCursor {
    pub incoming: Option<u32>,
    pub written: Option<u32>,
}

pub(crate) fn apply_usage_turn_locked(
    usage_path: &Path,
    lock_path: &Path,
    request: UsageTurnRequest,
) -> io::Result<UsageTurnCursor> {
    let lock = open_lock_file(lock_path)?;
    lock.lock_exclusive()?;
    let result = apply_usage_turn(usage_path, request);
    let _ = lock.unlock();
    result
}

fn apply_usage_turn(usage_path: &Path, request: UsageTurnRequest) -> io::Result<UsageTurnCursor> {
    let mut file = read_usage(usage_path)?
        .unwrap_or_else(|| SessionUsageFile::new(request.session_id.clone()));
    file.session_id = request.session_id;
    file.restore_apply_cursor(request.last_incoming_turn, request.last_written_turn);
    file.apply_turn(
        request.turn_number,
        request.ended_at,
        &request.live,
        request.prev_live.as_ref(),
    );
    let (incoming, written) = file.apply_cursor();
    let bytes = serde_json::to_vec_pretty(&file)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    crate::session::storage::write_bytes_atomic(usage_path, &bytes)?;
    Ok(UsageTurnCursor { incoming, written })
}

fn read_usage(path: &Path) -> io::Result<Option<SessionUsageFile>> {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if data.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn open_lock_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}
