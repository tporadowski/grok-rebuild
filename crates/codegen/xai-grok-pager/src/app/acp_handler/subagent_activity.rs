use super::*;

/// Update the activity label on a subagent's collapsed scrollback block.
/// Skips the write (and cache invalidation) when the label hasn't changed.
/// Most deltas keep the same label ("Responding" stays "Responding"), so the common case allocates nothing.
pub(super) fn sync_activity_label(
    scrollback: &mut crate::scrollback::state::ScrollbackState,
    entry_id: Option<crate::scrollback::entry::EntryId>,
    activity_label: Option<&str>,
) {
    if let Some(eid) = entry_id
        && let Some(entry) = scrollback.get_by_id_mut(eid)
        && let RenderBlock::Subagent(ref mut sb) = entry.block
        && sb.activity_label.as_deref() != activity_label
    {
        sb.activity_label = activity_label.map(str::to_owned);
        entry.invalidate_cache();
    }
}

/// Fan a subagent's computed activity label out to both places that show it, so the two can't drift.
/// Those are the collapsed scrollback block and the [`SubagentInfo`] backing the tasks pane and dashboard rows.
/// Once `finished` is set, only a clear (`None`) lands: buffered updates from the child race `SubagentFinished` and must not re-stamp the label.
pub(super) fn sync_subagent_activity(
    parent: &mut AgentView,
    child_key: &str,
    activity_label: Option<String>,
) {
    let Some(info) = parent.subagent_sessions.get_mut(child_key) else {
        return;
    };
    if info.is_finished() && activity_label.is_some() {
        return;
    }
    sync_activity_label(
        &mut parent.scrollback,
        info.attempt.scrollback_entry_id,
        activity_label.as_deref(),
    );
    info.attempt.activity_label = activity_label;
}

/// After a resume, a still-running foreground subagent has its transcript on disk and no live label.
/// Read that transcript and stamp the activity on the parent row. Background children stay deferred.
pub(crate) fn stamp_resumed_foreground_activity(parent: &mut AgentView) {
    let ids: Vec<String> = parent
        .subagent_sessions
        .iter()
        .filter(|(_, info)| {
            info.is_running()
                && !info.attempt.is_background
                && info.attempt.workflow_run_id.is_none()
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        if parent
            .subagent_sessions
            .get(&id)
            .is_some_and(|info| info.transcript.needs_replay())
        {
            let _ = crate::app::subagent::ensure_subagent_child_replayed(parent, &id);
        }
        let label = parent
            .subagent_views
            .get(&id)
            .and_then(|view| subagent_activity_label(view));
        sync_subagent_activity(parent, &id, label);
    }
}

/// Resolve a subagent child view's live activity into the display label [`sync_subagent_activity`] stamps.
/// A child that is busy between activities shows "Waiting".
pub(super) fn subagent_activity_label(child_view: &AgentView) -> Option<String> {
    match child_view.resolve_turn_activity() {
        Some(a) => Some(crate::app::subagent::format_activity_label(&a)),
        None if child_view.session.state.is_busy() => Some("Waiting".to_string()),
        None => None,
    }
}

/// Synthesize a finish for a stuck row when a kill found nothing live to stop (otherwise `pending_kill` times out and the row reads "running").
/// `status` is the real terminal status for an already-finished orphan, else `"cancelled"`.
/// When the child had already finished, the retained terminal status wins over the call's default (`cancelled`).
pub(crate) fn finalize_killed_subagent(
    app: &mut AppView,
    session_id: &acp::SessionId,
    subagent_id: &str,
    attempt_id: Option<&str>,
    status: &str,
) -> bool {
    let Some(SessionMatch::Root(agent_id)) = find_session_match(app, session_id) else {
        return false;
    };
    let Some(agent) = app.agents.get(&agent_id) else {
        return false;
    };
    let Some(info) = agent
        .subagent_sessions
        .values()
        .find(|info| info.subagent_id.as_ref() == subagent_id)
    else {
        return false;
    };
    if info.attempt.lifecycle.current_attempt_id() != attempt_id {
        return false;
    }
    let child_session_id = info.child_session_id.to_string();
    let attempt_id = attempt_id.map(str::to_owned);
    let update = if info.is_finished() {
        info.attempt
            .terminal_update(subagent_id, &child_session_id, status)
    } else {
        XaiSessionUpdate::SubagentFinished {
            subagent_id: subagent_id.to_owned(),
            attempt_id,
            child_session_id: child_session_id.clone(),
            status: status.to_owned(),
            error: None,
            tool_calls: 0,
            turns: 0,
            duration_ms: 0,
            tokens_used: 0,
            output: None,
            will_wake: false,
        }
    };
    let payload = SessionNotification {
        session_id: session_id.clone(),
        update,
        meta: None,
    };
    let Ok(params) = serde_json::value::to_raw_value(&payload) else {
        return false;
    };

    let notif = acp::ExtNotification::new("x.ai/session/update", params.into());
    handle_session_notification_with_origin(&notif, app, LifecycleOrigin::Reconciliation)
}
