//! A session another live Grok window has open can be forked instead of loaded.

use std::path::PathBuf;

use xai_grok_active_sessions::ActiveSession;

use crate::app::app_view::AppView;

pub(crate) const OTHER_WINDOW_FORK_NOTICE: &str =
    "This session is open in another window. Ctrl+F or /fork forks it.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingOtherWindowFork {
    pub session_id: String,
    pub cwd: Option<PathBuf>,
}

/// Another live process holds `session_id`. Our own pid does not count, and a dead pid does not count.
pub(crate) fn other_window_hold(
    rows: &[ActiveSession],
    session_id: &str,
    self_pid: u32,
) -> Option<u32> {
    rows.iter().find_map(|row| {
        if row.session_id.0.as_ref() != session_id || row.pid == self_pid {
            return None;
        }
        xai_grok_active_sessions::pid_is_alive(row.pid).then_some(row.pid)
    })
}

/// Record the fork offer and show it as a notice. Returns false when this window may load the session.
pub(crate) fn offer_fork_instead_of_load(
    app: &mut AppView,
    session_id: &str,
    cwd: Option<PathBuf>,
    rows: &[ActiveSession],
    self_pid: u32,
) -> bool {
    if other_window_hold(rows, session_id, self_pid).is_none() {
        return false;
    }
    app.pending_other_window_fork = Some(PendingOtherWindowFork {
        session_id: session_id.to_string(),
        cwd,
    });
    app.show_toast(OTHER_WINDOW_FORK_NOTICE);
    true
}

pub(crate) fn held_rows(session_id: &str) -> Vec<ActiveSession> {
    xai_grok_active_sessions::list_in(&xai_grok_config::grok_home())
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.session_id.0.as_ref() == session_id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn row(id: &str, pid: u32) -> ActiveSession {
        ActiveSession {
            session_id: agent_client_protocol::SessionId::new(id),
            pid,
            cwd: "/tmp".into(),
            opened_at: Utc::now(),
        }
    }

    #[test]
    fn other_window_hold_ignores_our_pid_and_a_dead_pid() {
        let live = std::process::id();
        let rows = vec![row("held", live)];
        assert_eq!(
            other_window_hold(&rows, "held", live.wrapping_add(1)),
            Some(live)
        );
        assert_eq!(other_window_hold(&rows, "held", live), None);
        assert_eq!(
            other_window_hold(&[row("held", u32::MAX)], "held", live),
            None
        );
        assert_eq!(other_window_hold(&rows, "other", live.wrapping_add(1)), None);
    }
}
