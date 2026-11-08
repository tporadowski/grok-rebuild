use pretty_assertions::assert_eq;

#[test]
fn an_active_goal_is_pending_until_it_reports_an_outcome() {
    let mut pending = std::collections::HashSet::new();
    super::apply_goal_background_update(&mut pending, "g1", "active");
    assert!(pending.contains(&super::BackgroundWork::Goal("g1".into())));
    assert!(super::goal_is_still_running("Active"));

    super::apply_goal_background_update(&mut pending, "g1", "complete");
    assert!(pending.is_empty());
    assert_eq!(
        super::format_goal_outcome("complete", "ship the port"),
        "Goal complete: ship the port"
    );
    assert_eq!(
        super::reducer::Lifecycle::GoalFinished {
            status: "complete".into(),
            objective: "ship the port".into(),
        }
        .plain_message(),
        "Goal complete: ship the port"
    );
}

#[test]
fn headless_goal_completion_does_not_print_a_later_pause() {
    let mut pending = std::collections::HashSet::new();
    let mut finished = std::collections::HashSet::new();

    assert!(
        super::record_headless_goal_update(&mut pending, &mut finished, "g1", "active", "ship")
            .is_none()
    );
    assert!(pending.contains(&super::BackgroundWork::Goal("g1".into())));
    assert!(super::headless_exit_pauses_goal(&finished, "g1"));

    let (status, objective) = super::record_headless_goal_update(
        &mut pending,
        &mut finished,
        "g1",
        "complete",
        "ship the port",
    )
    .expect("completion is announced");
    assert_eq!(
        super::reducer::Lifecycle::GoalFinished { status, objective }.plain_message(),
        "Goal complete: ship the port"
    );
    assert!(pending.is_empty());
    assert!(!super::headless_exit_pauses_goal(&finished, "g1"));

    for status in ["paused", "user_paused", "blocked", "Active"] {
        assert!(
            super::record_headless_goal_update(&mut pending, &mut finished, "g1", status, "ship")
                .is_none(),
            "{status} after completion must stay quiet"
        );
    }
    assert!(
        pending.is_empty(),
        "a stale active after completion must not arm /goal pause"
    );

    let (status, objective) =
        super::record_headless_goal_update(&mut pending, &mut finished, "g2", "paused", "")
            .expect("a goal that never completed still reports a pause");
    assert_eq!(
        super::reducer::Lifecycle::GoalFinished { status, objective }.plain_message(),
        "Goal paused."
    );
    assert!(super::headless_exit_pauses_goal(&finished, "g2"));
}

#[test]
fn a_background_wait_timeout_is_an_error() {
    let message = super::background_wait_timeout_message();
    assert!(message.contains("timed out"));
    assert!(message.contains("pending"));
}

#[test]
fn lifecycle_tracking_is_independent_of_wait_flag() {
    let mut pending = std::collections::HashSet::new();
    let mut completed = super::BackgroundLifecycleState::default();
    super::track_background_lifecycle(
        super::ExtEvent::TaskBackgrounded {
            task_id: "t1".into(),
            is_monitor: false,
        },
        &mut pending,
        &mut completed,
    );
    super::track_background_lifecycle(
        super::ExtEvent::SubagentSpawned {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(1),
        },
        &mut pending,
        &mut completed,
    );
    assert!(pending.contains(&super::BackgroundWork::Task("t1".into())));
    assert!(pending.contains(&super::BackgroundWork::Subagent("s1".into())));

    super::track_background_lifecycle(
        super::ExtEvent::TaskCompleted {
            task_id: "t1".into(),
        },
        &mut pending,
        &mut completed,
    );
    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(2),
        },
        &mut pending,
        &mut completed,
    );
    assert!(pending.is_empty());
}

#[test]
fn completion_before_backgrounded_never_rearms_pending() {
    let mut pending = std::collections::HashSet::new();
    let mut completed = super::BackgroundLifecycleState::default();
    super::track_background_lifecycle(
        super::ExtEvent::TaskCompleted {
            task_id: "t1".into(),
        },
        &mut pending,
        &mut completed,
    );
    super::track_background_lifecycle(
        super::ExtEvent::TaskBackgrounded {
            task_id: "t1".into(),
            is_monitor: false,
        },
        &mut pending,
        &mut completed,
    );
    assert!(pending.is_empty());
}

#[test]
fn duplicate_backgrounded_after_completion_stays_dead() {
    let mut pending = std::collections::HashSet::new();
    let mut completed = super::BackgroundLifecycleState::default();
    let bg = || super::ExtEvent::TaskBackgrounded {
        task_id: "t1".into(),
        is_monitor: false,
    };
    super::track_background_lifecycle(bg(), &mut pending, &mut completed);
    assert!(pending.contains(&super::BackgroundWork::Task("t1".into())));
    super::track_background_lifecycle(
        super::ExtEvent::TaskCompleted {
            task_id: "t1".into(),
        },
        &mut pending,
        &mut completed,
    );
    assert!(pending.is_empty());
    super::track_background_lifecycle(bg(), &mut pending, &mut completed);
    assert!(
        pending.is_empty(),
        "a backgrounded for an already-completed id must not re-arm pending"
    );
}

fn s1(state: &super::BackgroundLifecycleState) -> &crate::app::subagent::SubagentLifecycleState {
    let Some(s) = state.subagents.get("s1") else {
        panic!("missing subagent s1: {:?}", state.subagents.keys());
    };
    s
}

fn spawn_event(attempt_id: &str, event_seq: u64) -> super::ExtEvent {
    super::ExtEvent::SubagentSpawned {
        subagent_id: "s1".into(),
        attempt_id: Some(attempt_id.into()),
        event_seq: Some(event_seq),
    }
}

fn legacy_spawn_event(event_seq: Option<u64>) -> super::ExtEvent {
    super::ExtEvent::SubagentSpawned {
        subagent_id: "s1".into(),
        attempt_id: None,
        event_seq,
    }
}

fn legacy_finish_event(event_seq: Option<u64>) -> super::ExtEvent {
    super::ExtEvent::SubagentFinished {
        subagent_id: "s1".into(),
        attempt_id: None,
        event_seq,
    }
}

#[test]
fn newer_subagent_spawn_after_finish_rearms_pending() {
    let mut pending = std::collections::HashSet::new();
    let mut completed = super::BackgroundLifecycleState::default();
    let work = super::BackgroundWork::Subagent("s1".into());
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut completed);
    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(2),
        },
        &mut pending,
        &mut completed,
    );
    assert!(pending.is_empty());
    assert!(completed.subagents.contains_key("s1"));

    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut completed);
    assert!(pending.is_empty());
    assert!(completed.subagents.contains_key("s1"));

    super::track_background_lifecycle(spawn_event("at1.two", 3), &mut pending, &mut completed);
    assert!(pending.contains(&work));
    assert_eq!(s1(&completed).current_attempt_id(), Some("at1.two"));
    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(2),
        },
        &mut pending,
        &mut completed,
    );
    assert!(pending.contains(&work));
    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.two".into()),
            event_seq: Some(4),
        },
        &mut pending,
        &mut completed,
    );
    assert!(pending.is_empty());
    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(2),
        },
        &mut pending,
        &mut completed,
    );
    super::track_background_lifecycle(spawn_event("at1.two", 3), &mut pending, &mut completed);
    assert!(pending.is_empty());
    assert_eq!(s1(&completed).last_event_seq(), Some(4));
}

#[test]
fn retained_prior_finish_records_without_clearing_current_pending() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    let work = super::BackgroundWork::Subagent("s1".into());
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);
    super::track_background_lifecycle(spawn_event("at1.two", 3), &mut pending, &mut state);

    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(2),
        },
        &mut pending,
        &mut state,
    );

    assert!(s1(&state).is_attempt_finished("at1.one"));
    assert_eq!(s1(&state).current_attempt_id(), Some("at1.two"));
    assert!(!s1(&state).is_finished());
    assert!(pending.contains(&work));
}

#[test]
fn exact_pending_finish_discards_legacy_before_next_spawn() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    let work = super::BackgroundWork::Subagent("s1".into());
    super::track_background_lifecycle(legacy_finish_event(Some(2)), &mut pending, &mut state);
    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(3),
        },
        &mut pending,
        &mut state,
    );
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);

    assert!(s1(&state).is_attempt_finished("at1.one"));
    assert!(!s1(&state).retains_attempt(&crate::app::subagent::SubagentAttemptKey::Legacy));
    super::track_background_lifecycle(spawn_event("at1.two", 4), &mut pending, &mut state);
    assert_eq!(s1(&state).current_attempt_id(), Some("at1.two"));
    assert!(!s1(&state).is_finished());
    assert!(pending.contains(&work));
}

#[test]
fn legacy_finish_before_typed_spawn_does_not_leave_pending_work() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    super::track_background_lifecycle(legacy_finish_event(Some(2)), &mut pending, &mut state);

    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);

    assert_eq!(s1(&state).current_attempt_id(), Some("at1.one"));
    assert!(s1(&state).is_finished());
    assert!(pending.is_empty());
}

#[test]
fn legacy_finish_before_two_typed_spawns_rearms_only_the_second() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    let work = super::BackgroundWork::Subagent("s1".into());
    super::track_background_lifecycle(legacy_finish_event(Some(2)), &mut pending, &mut state);
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);

    super::track_background_lifecycle(spawn_event("at1.two", 3), &mut pending, &mut state);

    assert!(s1(&state).is_attempt_finished("at1.one"));
    assert_eq!(s1(&state).current_attempt_id(), Some("at1.two"));
    assert!(!s1(&state).is_finished());
    assert!(pending.contains(&work));
}

#[test]
fn legacy_finish_closes_the_only_typed_running_attempt() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    let work = super::BackgroundWork::Subagent("s1".into());
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);
    assert!(pending.contains(&work));

    super::track_background_lifecycle(legacy_finish_event(Some(2)), &mut pending, &mut state);

    assert!(s1(&state).is_finished());
    assert!(pending.is_empty());
}

#[test]
fn legacy_finish_closes_current_typed_attempt_after_wake() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    let work = super::BackgroundWork::Subagent("s1".into());
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);
    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(2),
        },
        &mut pending,
        &mut state,
    );
    super::track_background_lifecycle(spawn_event("at1.two", 3), &mut pending, &mut state);
    assert!(pending.contains(&work));

    super::track_background_lifecycle(legacy_finish_event(Some(4)), &mut pending, &mut state);

    assert_eq!(s1(&state).current_attempt_id(), Some("at1.two"));
    assert!(s1(&state).is_finished());
    assert!(pending.is_empty());
}

#[test]
fn legacy_finish_after_typed_completion_is_dropped() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);
    super::track_background_lifecycle(
        super::ExtEvent::SubagentFinished {
            subagent_id: "s1".into(),
            attempt_id: Some("at1.one".into()),
            event_seq: Some(2),
        },
        &mut pending,
        &mut state,
    );

    super::track_background_lifecycle(legacy_finish_event(Some(3)), &mut pending, &mut state);

    assert_eq!(s1(&state).current_attempt_id(), Some("at1.one"));
    assert!(s1(&state).is_finished());
    assert_eq!(s1(&state).last_event_seq(), Some(2));
    assert!(!s1(&state).retains_attempt(&crate::app::subagent::SubagentAttemptKey::Legacy));
    assert!(pending.is_empty());
}

#[test]
fn finished_legacy_duplicate_spawn_does_not_rearm_pending() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    super::track_background_lifecycle(legacy_spawn_event(None), &mut pending, &mut state);
    super::track_background_lifecycle(legacy_finish_event(None), &mut pending, &mut state);
    super::track_background_lifecycle(legacy_spawn_event(None), &mut pending, &mut state);

    assert_eq!(s1(&state).current_attempt_id(), None);
    assert!(s1(&state).is_finished());
    assert!(pending.is_empty());
}

#[test]
fn reading_finished_output_in_the_turn_releases_the_headless_wait() {
    let mut pending = std::collections::HashSet::new();
    let mut state = super::BackgroundLifecycleState::default();
    let task = super::BackgroundWork::Task("t1".into());
    let child = super::BackgroundWork::Subagent("s1".into());

    super::track_background_lifecycle(
        super::ExtEvent::TaskBackgrounded {
            task_id: "t1".into(),
            is_monitor: false,
        },
        &mut pending,
        &mut state,
    );
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);
    assert!(pending.contains(&task));
    assert!(pending.contains(&child));

    super::note_terminal_output_read(
        &mut pending,
        &mut state,
        Some(&serde_json::json!({
            "type": "TaskOutput",
            "Result": { "task_id": "t1", "status": "running", "output": "partial" }
        })),
    );
    assert!(
        pending.contains(&task),
        "a read of a still-running task keeps the wait"
    );

    super::note_terminal_output_read(
        &mut pending,
        &mut state,
        Some(&serde_json::json!({
            "type": "TaskOutput",
            "Result": { "task_id": "t1", "status": "completed", "output": "done" }
        })),
    );
    assert!(!pending.contains(&task));
    super::track_background_lifecycle(
        super::ExtEvent::TaskBackgrounded {
            task_id: "t1".into(),
            is_monitor: false,
        },
        &mut pending,
        &mut state,
    );
    assert!(
        !pending.contains(&task),
        "a later backgrounded notice must not re-arm a task whose output was read"
    );

    super::note_terminal_output_read(
        &mut pending,
        &mut state,
        Some(&serde_json::json!({
            "type": "TaskOutput",
            "MultiResult": {
                "results": [
                    { "task_id": "s1", "status": "completed", "output": "answer" },
                    { "task_id": "still", "status": "running" }
                ]
            }
        })),
    );
    assert!(!pending.contains(&child));
    assert!(s1(&state).is_finished());
    super::track_background_lifecycle(spawn_event("at1.one", 1), &mut pending, &mut state);
    assert!(
        !pending.contains(&child),
        "replaying the finished attempt must not re-arm the wait"
    );
    super::track_background_lifecycle(spawn_event("at1.two", 3), &mut pending, &mut state);
    assert!(
        pending.contains(&child),
        "a new attempt after the read is still waited on"
    );

    super::track_background_lifecycle(
        super::ExtEvent::SubagentSpawned {
            subagent_id: "child-2".into(),
            attempt_id: Some("at2.one".into()),
            event_seq: Some(1),
        },
        &mut pending,
        &mut state,
    );
    super::track_background_lifecycle(
        super::ExtEvent::TaskBackgrounded {
            task_id: "awaited".into(),
            is_monitor: false,
        },
        &mut pending,
        &mut state,
    );
    super::note_terminal_output_read(
        &mut pending,
        &mut state,
        Some(&serde_json::json!({
            "type": "SubagentCompleted",
            "subagent_id": "child-2",
            "output": "joined"
        })),
    );
    super::note_terminal_output_read(
        &mut pending,
        &mut state,
        Some(&serde_json::json!({
            "type": "Text",
            "text": "Task completed.",
            "consumed_completion_task_id": "awaited"
        })),
    );
    assert!(!pending.contains(&super::BackgroundWork::Subagent("child-2".into())));
    assert!(!pending.contains(&super::BackgroundWork::Task("awaited".into())));
}
