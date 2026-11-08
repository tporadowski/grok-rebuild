pub mod config;
pub mod focus;
pub mod hooks;
pub mod program_status;
pub mod progress;
pub mod protocol;
pub mod sleep;
pub mod title;
pub mod tmux;

use std::time::{Duration, Instant};

use crate::render::draw::EscapeWriter;

/// Ghostty resets the OSC 9;4 progress indicator after ~15 s of silence.
/// Re-send the sequence at this interval to keep it alive.
const PROGRESS_KEEPALIVE: Duration = Duration::from_secs(5);

pub use config::{
    NotificationCondition, NotificationConfig, NotificationEventKind, NotificationHook,
    NotificationMethod, ProgramStatusConfig, TitleConfig, TitleItem,
};
pub use title::TitleState;

pub struct NotificationEvent {
    pub kind: NotificationEventKind,
    pub title: String,
    pub body: String,
    pub session_id: Option<String>,
}

pub struct NotificationService {
    config: NotificationConfig,
    pub focus_tracker: focus::FocusTracker,
    pub sleep_inhibitor: sleep::SleepInhibitor,
    title_manager: title::TitleManager,
    protocol: protocol::NotificationProtocol,
    terminal_ctx: &'static crate::terminal::TerminalContext,
    /// Whether the OSC 9;4 progress indicator is currently active.
    progress_active: bool,
    /// Last time the progress bar escape was emitted (keep-alive clock).
    progress_last_sent: Option<Instant>,
    /// Whether we have already fired an `ApprovalRequired` terminal notification for the current batch of queued permissions.
    /// Set to `true` after the first notification; cleared via [`clear_permission_notification`] when the queue drains to empty.
    permission_notified: bool,
    /// Out-of-band escape queue for notification/shutdown escapes; see [`EscapeWriter`](crate::render::draw::EscapeWriter).
    escape_writer: EscapeWriter,
    program_status: program_status::ProgramStatusPublisher,
}

impl NotificationService {
    pub fn new(config: NotificationConfig, escape_writer: EscapeWriter) -> Self {
        let terminal_ctx = crate::terminal::terminal_context();
        let protocol = resolve_protocol(config.method, terminal_ctx);
        let focus_tracker = focus::FocusTracker::new(
            config.idle_threshold_secs,
            config.session_recap_threshold_secs,
        );
        let sleep_inhibitor = sleep::SleepInhibitor::new(config.sleep_prevention);
        let title_manager = title::TitleManager::new(&config.title);
        Self {
            config,
            focus_tracker,
            sleep_inhibitor,
            title_manager,
            protocol,
            terminal_ctx,
            progress_active: false,
            progress_last_sent: None,
            permission_notified: false,
            escape_writer,
            program_status: program_status::ProgramStatusPublisher::default(),
        }
    }

    /// OSC 7501 reports for the open sessions. Empty when the picture has not changed.
    pub(crate) fn program_status_escapes(
        &mut self,
        sessions: &[program_status::SessionSnap],
        focused: bool,
    ) -> Option<String> {
        let sequences = self.program_status.update(
            sessions,
            self.config.program_status.children,
            focused,
        );
        self.wrap_program_status(sequences)
    }

    pub(crate) fn note_program_failure(&mut self, session_key: &str) {
        self.program_status.note_failed(session_key);
    }

    fn wrap_program_status(&self, sequences: Vec<String>) -> Option<String> {
        if sequences.is_empty() {
            return None;
        }
        let mut buf = String::new();
        for sequence in sequences {
            if tmux::passthrough_available(self.terminal_ctx) {
                buf.push_str(&tmux::tmux_passthrough(&sequence));
            } else {
                buf.push_str(&sequence);
            }
        }
        Some(buf)
    }

    pub fn config(&self) -> &NotificationConfig {
        &self.config
    }

    pub fn protocol(&self) -> protocol::NotificationProtocol {
        self.protocol
    }

    fn is_event_enabled(&self, kind: &NotificationEventKind) -> bool {
        self.config.events.contains(kind)
    }

    /// Custom `[[ui.notifications.hooks]]` commands run on their own event list.
    /// An empty list matches every kind. This is independent of the terminal `events` filter.
    fn run_matching_hooks(&self, event: &NotificationEvent) {
        let unfocused = !self.focus_tracker.is_focused();
        for hook in &self.config.hooks {
            if hooks::hook_matches(hook, event.kind, unfocused) {
                hooks::run_hook(hook, event);
            }
        }
    }

    fn should_emit_terminal(&self) -> bool {
        match self.config.condition {
            NotificationCondition::Always => true,
            NotificationCondition::Unfocused => self.focus_tracker.should_notify(),
            NotificationCondition::Never => false,
        }
    }

    /// Notifications bypass the frame pipeline: rare one-shot events that must not wait for the next draw (16ms away,
    /// or indefinitely while a frame ack is outstanding). For `ApprovalRequired` events, the caller must check
    /// [`should_suppress_permission_notification`] first.
    pub fn notify(&self, event: NotificationEvent) {
        self.run_matching_hooks(&event);
        if !self.is_event_enabled(&event.kind) {
            return;
        }
        if self.should_emit_terminal() {
            protocol::emit_notification(
                self.protocol,
                &event.title,
                &event.body,
                self.terminal_ctx,
                &self.escape_writer,
            );
            xai_grok_telemetry::session_ctx::log_event(
                xai_grok_telemetry::events::NotificationEmitted {
                    protocol: self.protocol.into(),
                    event_kind: event.kind.into(),
                    was_focused: self.focus_tracker.is_focused(),
                },
            );
        }
    }

    /// Build escape sequences to set the title and progress bar to idle without writing to stderr.
    /// The caller can route these through the frame pipeline (`pending_notification_escapes`) so they go through the writer thread.
    /// They are then ordered correctly relative to previous frames that may still carry the busy title.
    pub fn build_idle_escapes(&mut self, state: &title::TitleState<'_>) -> Option<String> {
        let mut buf = String::new();

        if self.config.title.enabled
            && let Some(esc) = self.title_manager.update(state)
        {
            buf.push_str(&esc);
        }

        if !state.is_busy {
            self.clear_progress_into(&mut buf);
        }

        if buf.is_empty() { None } else { Some(buf) }
    }

    /// Advance the tab title and progress bar state.
    /// Returns escape sequences to emit (title and progress) as a single `String`, or `None` if nothing changed.
    /// The caller should route these bytes through the frame pipeline's `post_flush_escapes`.
    pub fn on_tick(&mut self, state: &title::TitleState<'_>) -> Option<String> {
        let mut buf = String::new();

        if self.config.title.enabled
            && let Some(title_esc) = self.title_manager.update(state)
        {
            buf.push_str(&title_esc);
        }

        // Drive OSC 9;4 tab progress bar
        // Ghostty resets the indicator after ~15 s of silence, so we re-send it as a keep-alive
        if self.config.progress_bar {
            let should_be_active = state.is_busy;
            if should_be_active {
                let needs_emit = !self.progress_active
                    || self
                        .progress_last_sent
                        .is_none_or(|t| t.elapsed() >= PROGRESS_KEEPALIVE);
                if needs_emit {
                    if let Some(esc) = progress::build_progress_escape(
                        progress::ProgressState::Indeterminate,
                        self.terminal_ctx,
                    ) {
                        buf.push_str(&esc);
                    }
                    self.progress_active = true;
                    self.progress_last_sent = Some(Instant::now());
                }
            } else {
                self.clear_progress_into(&mut buf);
            }
        }

        if buf.is_empty() { None } else { Some(buf) }
    }

    /// Reset the tab title back to "grok" and clear the progress bar so neither lingers after exit. Enqueued, never
    /// inline: `/quit` can land while the writer is parked holding the stderr lock, and the queue orders the reset
    /// after any still-queued busy-title escape.
    pub fn shutdown(&mut self) {
        let mut buf = self.title_manager.reset();
        self.clear_progress_into(&mut buf);
        let clear = self.program_status.clear_all();
        if let Some(clear) = self.wrap_program_status(vec![clear]) {
            buf.push_str(&clear);
        }
        self.escape_writer.emit(buf);
    }

    /// Returns `true` if a terminal notification for `ApprovalRequired` has already been emitted and should not be repeated.
    pub fn should_suppress_permission_notification(&self) -> bool {
        self.permission_notified
    }

    /// Record that an `ApprovalRequired` notification has been emitted.
    pub fn mark_permission_notified(&mut self) {
        self.permission_notified = true;
    }

    /// Reset the permission notification flag.
    /// Call this when the permission queue drains to empty.
    pub fn clear_permission_notification(&mut self) {
        self.permission_notified = false;
    }

    fn clear_progress_into(&mut self, buf: &mut String) {
        if !self.progress_active {
            return;
        }
        if let Some(esc) =
            progress::build_progress_escape(progress::ProgressState::Clear, self.terminal_ctx)
        {
            buf.push_str(&esc);
        }
        self.progress_active = false;
        self.progress_last_sent = None;
    }

    /// Whether the OSC 9;4 progress indicator is currently considered active.
    /// Test-only: production callers drive progress exclusively via [`Self::on_tick`] / [`Self::build_idle_escapes`].
    #[cfg(test)]
    pub(crate) fn is_progress_active(&self) -> bool {
        self.progress_active
    }

    #[cfg(test)]
    fn new_for_test(config: NotificationConfig) -> Self {
        let terminal_ctx = crate::terminal::terminal_context();
        let focus_tracker = focus::FocusTracker::new(
            config.idle_threshold_secs,
            config.session_recap_threshold_secs,
        );
        let sleep_inhibitor = sleep::SleepInhibitor::new(config.sleep_prevention);
        let title_manager = title::TitleManager::new(&config.title);
        Self {
            config,
            focus_tracker,
            sleep_inhibitor,
            title_manager,
            protocol: protocol::NotificationProtocol::None,
            terminal_ctx,
            progress_active: false,
            progress_last_sent: None,
            permission_notified: false,
            escape_writer: EscapeWriter::disconnected(),
            program_status: program_status::ProgramStatusPublisher::default(),
        }
    }
}

fn resolve_protocol(
    method: NotificationMethod,
    ctx: &crate::terminal::TerminalContext,
) -> protocol::NotificationProtocol {
    match method {
        NotificationMethod::Auto => protocol::select_protocol(ctx),
        NotificationMethod::Osc9 => protocol::NotificationProtocol::Osc9,
        NotificationMethod::Osc99 => protocol::NotificationProtocol::Osc99,
        NotificationMethod::Osc777 => protocol::NotificationProtocol::Osc777,
        NotificationMethod::Bel => protocol::NotificationProtocol::Bel,
        NotificationMethod::None => protocol::NotificationProtocol::None,
    }
}

/// Load `NotificationConfig` from a raw TOML config value.
///
/// Looks for `[ui.notifications]`; falls back to defaults if absent or malformed.
pub fn load_notification_config(raw_config: &toml::Value) -> NotificationConfig {
    raw_config
        .get("ui")
        .and_then(|ui| ui.get("notifications"))
        .and_then(|n| toml::to_string(n).ok())
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::draw::WriterSync;
    use crate::terminal::{TerminalContext, TerminalName};

    #[test]
    fn resolve_protocol_auto_delegates_to_select() {
        let ctx = TerminalContext {
            brand: TerminalName::Kitty,
            ..Default::default()
        };
        assert_eq!(
            resolve_protocol(NotificationMethod::Auto, &ctx),
            protocol::NotificationProtocol::Osc99,
        );
    }

    #[test]
    fn resolve_protocol_explicit_overrides_auto_detection() {
        let ctx = TerminalContext {
            brand: TerminalName::Kitty,
            ..Default::default()
        };
        assert_eq!(
            resolve_protocol(NotificationMethod::Bel, &ctx),
            protocol::NotificationProtocol::Bel,
        );
    }

    #[test]
    fn resolve_protocol_all_explicit_methods() {
        let ctx = TerminalContext::default();
        let cases = [
            (
                NotificationMethod::Osc9,
                protocol::NotificationProtocol::Osc9,
            ),
            (
                NotificationMethod::Osc99,
                protocol::NotificationProtocol::Osc99,
            ),
            (
                NotificationMethod::Osc777,
                protocol::NotificationProtocol::Osc777,
            ),
            (NotificationMethod::Bel, protocol::NotificationProtocol::Bel),
            (
                NotificationMethod::None,
                protocol::NotificationProtocol::None,
            ),
        ];
        for (method, expected) in cases {
            assert_eq!(
                resolve_protocol(method, &ctx),
                expected,
                "mismatch for {method:?}"
            );
        }
    }

    #[test]
    fn is_event_enabled_filters_absent_kinds() {
        let svc = NotificationService::new_for_test(NotificationConfig {
            events: vec![NotificationEventKind::TurnComplete],
            condition: NotificationCondition::Always,
            ..Default::default()
        });
        assert!(!svc.is_event_enabled(&NotificationEventKind::SessionReady));
        assert!(!svc.is_event_enabled(&NotificationEventKind::AgentError));
    }

    #[test]
    fn is_event_enabled_allows_present_kinds() {
        let svc = NotificationService::new_for_test(NotificationConfig {
            events: vec![
                NotificationEventKind::TurnComplete,
                NotificationEventKind::AgentError,
            ],
            condition: NotificationCondition::Always,
            ..Default::default()
        });
        assert!(svc.is_event_enabled(&NotificationEventKind::TurnComplete));
        assert!(svc.is_event_enabled(&NotificationEventKind::AgentError));
    }

    #[test]
    fn should_emit_terminal_never_blocks() {
        let svc = NotificationService::new_for_test(NotificationConfig {
            condition: NotificationCondition::Never,
            ..Default::default()
        });
        assert!(!svc.should_emit_terminal());
    }

    #[test]
    fn should_emit_terminal_always_fires_when_focused() {
        let svc = NotificationService::new_for_test(NotificationConfig {
            condition: NotificationCondition::Always,
            ..Default::default()
        });
        assert!(svc.focus_tracker.is_focused());
        assert!(svc.should_emit_terminal());
    }

    #[test]
    fn should_emit_terminal_unfocused_blocks_when_focused() {
        let svc = NotificationService::new_for_test(NotificationConfig {
            condition: NotificationCondition::Unfocused,
            idle_threshold_secs: 0,
            ..Default::default()
        });
        assert!(svc.focus_tracker.is_focused());
        assert!(!svc.should_emit_terminal());
    }

    #[test]
    fn should_emit_terminal_unfocused_fires_when_idle() {
        let svc = NotificationService::new_for_test(NotificationConfig {
            condition: NotificationCondition::Unfocused,
            idle_threshold_secs: 0,
            ..Default::default()
        });
        svc.focus_tracker.on_focus_lost();
        assert!(svc.should_emit_terminal());
    }

    #[test]
    fn should_emit_terminal_unfocused_respects_idle_threshold() {
        let svc = NotificationService::new_for_test(NotificationConfig {
            condition: NotificationCondition::Unfocused,
            idle_threshold_secs: 60,
            ..Default::default()
        });
        svc.focus_tracker.on_focus_lost();
        assert!(!svc.should_emit_terminal());
    }

    #[test]
    fn should_emit_terminal_refocus_stops_notifications() {
        let svc = NotificationService::new_for_test(NotificationConfig {
            condition: NotificationCondition::Unfocused,
            idle_threshold_secs: 0,
            ..Default::default()
        });
        svc.focus_tracker.on_focus_lost();
        assert!(svc.should_emit_terminal());
        svc.focus_tracker.on_focus_gained();
        assert!(!svc.should_emit_terminal());
    }

    #[test]
    fn notify_enqueues_nothing_with_none_protocol() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            events: vec![NotificationEventKind::TurnComplete],
            condition: NotificationCondition::Always,
            ..Default::default()
        });
        svc.escape_writer = EscapeWriter::new(tx, WriterSync::new());
        svc.notify(NotificationEvent {
            kind: NotificationEventKind::TurnComplete,
            title: "Grok".into(),
            body: "Turn complete".into(),
            session_id: Some("test-session".into()),
        });
        assert!(
            rx.try_recv().is_err(),
            "protocol None must enqueue no payload"
        );
    }

    #[test]
    fn matching_notification_hook_runs_from_notify() {
        let dir = tempfile::tempdir().unwrap();
        let matched = dir.path().join("matched");
        let skipped = dir.path().join("skipped");
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            events: vec![NotificationEventKind::ApprovalRequired],
            condition: NotificationCondition::Never,
            hooks: vec![
                NotificationHook {
                    command: format!("touch {}", matched.display()),
                    events: vec![NotificationEventKind::TurnComplete],
                    only_unfocused: false,
                    timeout_secs: 5,
                },
                NotificationHook {
                    command: format!("touch {}", skipped.display()),
                    events: vec![NotificationEventKind::AgentError],
                    only_unfocused: false,
                    timeout_secs: 5,
                },
            ],
            ..Default::default()
        });
        svc.escape_writer = EscapeWriter::new(tx, WriterSync::new());
        svc.notify(NotificationEvent {
            kind: NotificationEventKind::TurnComplete,
            title: "Grok".into(),
            body: "Turn complete".into(),
            session_id: None,
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !matched.exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "matching notification hook did not run"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            !skipped.exists(),
            "a hook for a different event must not run"
        );
    }

    #[test]
    fn unfocused_notification_hook_waits_until_focus_is_lost() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("ran");
        let svc = NotificationService::new_for_test(NotificationConfig {
            hooks: vec![NotificationHook {
                command: format!("touch {}", marker.display()),
                events: vec![],
                only_unfocused: true,
                timeout_secs: 5,
            }],
            ..Default::default()
        });
        let event = NotificationEvent {
            kind: NotificationEventKind::SessionReady,
            title: "Grok".into(),
            body: "ready".into(),
            session_id: None,
        };
        svc.notify(NotificationEvent {
            kind: event.kind,
            title: event.title.clone(),
            body: event.body.clone(),
            session_id: event.session_id.clone(),
        });
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(
            !marker.exists(),
            "only_unfocused hook must not run while focused"
        );
        svc.focus_tracker.on_focus_lost();
        svc.notify(event);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !marker.exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "only_unfocused hook did not run after focus was lost"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn notify_skips_filtered_event() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            events: vec![NotificationEventKind::TurnComplete],
            condition: NotificationCondition::Always,
            ..Default::default()
        });
        svc.escape_writer = EscapeWriter::new(tx, WriterSync::new());
        svc.notify(NotificationEvent {
            kind: NotificationEventKind::SessionReady,
            title: "Grok".into(),
            body: "Session ready".into(),
            session_id: None,
        });
        assert!(
            rx.try_recv().is_err(),
            "a filtered event kind must enqueue no payload"
        );
    }

    #[test]
    fn load_parses_valid_ui_notifications() {
        let raw: toml::Value = toml::from_str(
            r#"
            [ui.notifications]
            method = "osc99"
            condition = "always"
            idle_threshold_secs = 15
            "#,
        )
        .unwrap();

        let config = load_notification_config(&raw);
        assert_eq!(config.method, NotificationMethod::Osc99);
        assert_eq!(config.condition, NotificationCondition::Always);
        assert_eq!(config.idle_threshold_secs, 15);
    }

    #[test]
    fn load_returns_defaults_when_ui_key_missing() {
        let raw: toml::Value = toml::from_str("[other]\nkey = 1\n").unwrap();
        assert_eq!(
            load_notification_config(&raw),
            NotificationConfig::default()
        );
    }

    #[test]
    fn load_returns_defaults_when_notifications_key_missing() {
        let raw: toml::Value = toml::from_str("[ui]\ntheme = \"dark\"\n").unwrap();
        assert_eq!(
            load_notification_config(&raw),
            NotificationConfig::default()
        );
    }

    #[test]
    fn load_returns_defaults_for_malformed_notifications() {
        let raw: toml::Value = toml::from_str("[ui]\nnotifications = \"not-a-table\"\n").unwrap();
        assert_eq!(
            load_notification_config(&raw),
            NotificationConfig::default()
        );
    }

    #[test]
    fn load_returns_defaults_for_empty_config() {
        let raw: toml::Value = toml::from_str("").unwrap();
        assert_eq!(
            load_notification_config(&raw),
            NotificationConfig::default()
        );
    }

    fn make_title_state(is_busy: bool) -> title::TitleState<'static> {
        title::TitleState {
            session_name: None,
            model: None,
            activity: None,
            has_pending_permissions: false,
            cwd: None,
            turn_elapsed: None,
            is_busy,
            focused: true,
        }
    }

    #[test]
    fn progress_activates_when_busy_without_activity() {
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            progress_bar: true,
            ..Default::default()
        });
        assert!(!svc.is_progress_active());
        svc.on_tick(&make_title_state(true));
        assert!(svc.is_progress_active());
    }

    #[test]
    fn progress_clears_when_idle_after_busy() {
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            progress_bar: true,
            ..Default::default()
        });
        svc.on_tick(&make_title_state(true));
        assert!(svc.is_progress_active());

        svc.on_tick(&make_title_state(false));
        assert!(!svc.is_progress_active());
    }

    #[test]
    fn progress_keepalive_re_emits_after_interval() {
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            progress_bar: true,
            ..Default::default()
        });

        let _result = svc.on_tick(&make_title_state(true));
        assert!(svc.is_progress_active());
        // build_progress_escape returns None for unsupported brands, but the flag still flips; this locks keep-alive timing.
        let first_sent = svc.progress_last_sent;
        assert!(first_sent.is_some());

        // Immediately following ticks should NOT refresh (interval not elapsed).
        let _result = svc.on_tick(&make_title_state(true));
        assert_eq!(svc.progress_last_sent, first_sent);

        // Simulate the keep-alive interval elapsing.
        svc.progress_last_sent =
            Some(Instant::now() - PROGRESS_KEEPALIVE - std::time::Duration::from_millis(1));
        svc.on_tick(&make_title_state(true));
        assert!(svc.progress_last_sent.unwrap() > first_sent.unwrap());
    }

    #[test]
    fn progress_keepalive_clears_timestamp_on_idle() {
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            progress_bar: true,
            ..Default::default()
        });
        svc.on_tick(&make_title_state(true));
        assert!(svc.progress_last_sent.is_some());

        svc.on_tick(&make_title_state(false));
        assert!(!svc.is_progress_active());
        assert!(svc.progress_last_sent.is_none());
    }

    #[test]
    fn progress_disabled_when_config_off() {
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            progress_bar: false,
            ..Default::default()
        });
        svc.on_tick(&make_title_state(true));
        assert!(!svc.is_progress_active());
    }

    #[test]
    fn shutdown_clears_active_progress() {
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            progress_bar: true,
            ..Default::default()
        });
        svc.on_tick(&make_title_state(true));
        assert!(svc.is_progress_active());

        svc.shutdown();
        assert!(!svc.is_progress_active());
    }

    /// Shutdown's title/progress resets must ride the writer queue, never an inline write.
    #[test]
    fn shutdown_enqueues_resets_on_the_writer_queue() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            progress_bar: true,
            ..Default::default()
        });
        svc.escape_writer = EscapeWriter::new(tx, WriterSync::new());
        svc.on_tick(&make_title_state(true));

        svc.shutdown();

        assert!(!svc.is_progress_active());
        let payload = rx
            .try_recv()
            .expect("shutdown escapes must ride the writer queue");
        assert!(
            String::from_utf8_lossy(payload.data()).contains("grok"),
            "expected the title reset in the queued escape"
        );
        assert!(rx.try_recv().is_err(), "one combined payload expected");
    }

    #[test]
    fn build_idle_escapes_clears_progress_and_title() {
        let mut svc = NotificationService::new_for_test(NotificationConfig {
            progress_bar: true,
            ..Default::default()
        });

        svc.on_tick(&make_title_state(true));
        assert!(svc.is_progress_active());
        assert!(svc.progress_last_sent.is_some());

        svc.build_idle_escapes(&make_title_state(false));
        assert!(!svc.is_progress_active());
        assert!(svc.progress_last_sent.is_none());
    }

    #[test]
    fn permission_suppression_lifecycle() {
        let mut svc = NotificationService::new_for_test(NotificationConfig::default());

        // Initially not suppressed: first permission should fire
        assert!(!svc.should_suppress_permission_notification());

        // After marking, subsequent notifications are suppressed.
        svc.mark_permission_notified();
        assert!(svc.should_suppress_permission_notification());

        // Clearing (queue drained) allows the next batch to fire.
        svc.clear_permission_notification();
        assert!(!svc.should_suppress_permission_notification());
    }
}
