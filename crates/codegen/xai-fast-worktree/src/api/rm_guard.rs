//! Why `grok worktree rm` without `-f` must stop.
//!
//! In-use worktrees (a live creator pid, a process whose cwd is inside the
//! worktree, or an open session whose cwd is) and worktrees with unsaved
//! changes are refused. `-f` skips this.

use std::path::Path;
use std::time::Duration;

use crate::git::{KeepReason, Reclaim};

/// `Some(message)` when removal without `-f` must fail.
#[cfg(feature = "metadata")]
pub fn refuse_unforced_removal(path: &Path) -> Option<String> {
    match worktree_in_use(path) {
        InUse::Session => {
            return Some(
                "worktree is in use by an open session; pass -f to remove it anyway".to_string(),
            );
        }
        InUse::Process => {
            return Some(
                "worktree is in use by a running process; pass -f to remove it anyway".to_string(),
            );
        }
        InUse::SessionUnreadable => {
            return Some(
                "could not check whether an open session is using this worktree; pass -f to remove it anyway"
                    .to_string(),
            );
        }
        InUse::No => {}
    }
    if !path.exists() {
        return None;
    }
    match crate::git::reclaimable_within(path, None, Duration::from_secs(15)) {
        Reclaim::Keep(KeepReason::Dirty | KeepReason::HiddenFromStatus) => {
            Some("worktree has unsaved changes; pass -f to remove it anyway".to_string())
        }
        Reclaim::Keep(KeepReason::CheckFailed | KeepReason::GateTimedOut) => Some(
            "could not check the worktree for unsaved changes; pass -f to remove it anyway"
                .to_string(),
        ),
        _ => None,
    }
}

#[cfg(feature = "metadata")]
enum InUse {
    No,
    Session,
    Process,
    SessionUnreadable,
}

#[cfg(feature = "metadata")]
fn worktree_in_use(path: &Path) -> InUse {
    if let Ok(db) = crate::db::WorktreeDb::open_default() {
        let key = path.display().to_string();
        if let Ok(Some(rec)) = db.get(&key) {
            if rec.creator_pid.is_some_and(super::gc::is_pid_alive) {
                return InUse::Process;
            }
        }
    }
    if path.exists() {
        match super::gc::live_session_cwds() {
            super::gc::SessionCwdScan::Unreadable => return InUse::SessionUnreadable,
            super::gc::SessionCwdScan::Ok(cwds) => {
                let worktree = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
                if super::gc::cwd_within(&worktree, &cwds) {
                    return InUse::Session;
                }
            }
        }
    }
    let Ok(canon) = dunce::canonicalize(path) else {
        return InUse::No;
    };
    match super::gc::live_process_cwds() {
        super::gc::LiveCwdScan::Ok(cwds) if cwds.iter().any(|cwd| cwd_inside(cwd, &canon)) => {
            InUse::Process
        }
        _ => InUse::No,
    }
}

#[cfg(feature = "metadata")]
fn cwd_inside(cwd: &Path, worktree: &Path) -> bool {
    let cwd = dunce::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    cwd == worktree || cwd.starts_with(worktree)
}

#[cfg(all(test, feature = "metadata"))]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .status()
            .expect("git");
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn dirty_worktree_is_refused_without_force() {
        let tmp = tempfile::tempdir().unwrap();
        git(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("dirty.txt"), b"unsaved").unwrap();
        let reason = refuse_unforced_removal(tmp.path());
        assert!(
            reason.as_deref().is_some_and(|r| r.contains("unsaved")),
            "dirty tree must be refused, got {reason:?}"
        );
    }

    #[test]
    fn clean_worktree_is_not_refused() {
        let tmp = tempfile::tempdir().unwrap();
        git(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("ok.txt"), b"saved").unwrap();
        git(tmp.path(), &["add", "ok.txt"]);
        git(tmp.path(), &["commit", "-m", "init"]);
        assert!(refuse_unforced_removal(tmp.path()).is_none());
    }

    #[test]
    fn missing_path_is_not_refused() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(refuse_unforced_removal(&tmp.path().join("gone")).is_none());
    }

    #[test]
    fn unreadable_git_dir_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let git_dir = tmp.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        let mut locked = std::fs::metadata(&git_dir).unwrap().permissions();
        locked.set_mode(0);
        std::fs::set_permissions(&git_dir, locked.clone()).unwrap();
        let reason = refuse_unforced_removal(tmp.path());
        locked.set_mode(0o755);
        std::fs::set_permissions(&git_dir, locked).unwrap();
        let reason = reason.expect("unreadable .git must be refused");
        assert!(
            reason.contains("could not check"),
            "check failure must refuse, got {reason}"
        );
    }
}
