//! Headless single-turn mode (`grok -p "prompt"`).
//!
//! Runs the agent in-process and drives the ACP lifecycle (init, auth, session, prompt).
//! Streams to stdout and exits via `CancellationToken`.
use crate::acp::model_state::{EffortTokenError, ModelState};
use crate::acp::spawn::{AgentShutdownGuard, SpawnedAgent, spawn_grok_shell};
use crate::app::prompt_ack::{PromptAckDeadlines, PromptAckWatch};
use crate::app::subagent::{
    SubagentLifecycleEffect, SubagentLifecycleReduction, SubagentLifecycleState,
    SubagentLifecycleTransition,
};
use crate::app::worktree_session::{
    WorktreeSpec, create_worktree, new_worktree_id, note_orphaned_worktree,
    resume_session_into_worktree,
};
use crate::best_effort_stderr::eprint_line;
use crate::client_identity::{HEADLESS_CLIENT_TYPE, PAGER_CLIENT_VERSION};
use crate::headless::reducer::{
    Lifecycle, Reducer, SessionContext, StreamEvent, TurnEnd, map_session_update, reducer_for,
};
use agent_client_protocol as acp;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use xai_acp_lib::{AcpAgentTx, AcpClientMessageBox, AcpClientRx, acp_send};
use xai_grok_shell::agent::auth_method::AuthMethodKind;
use xai_grok_shell::agent::config::Config as AgentConfig;
use xai_grok_shell::extensions::memory::MemoryFlushResponse;
use xai_grok_shell::extensions::task::{CancelSubagentRequest, KillTaskRequest};
use xai_grok_shell::sampling::error::{
    RATE_LIMITED_ERROR_CODE, error_detail_from_data, format_rate_limited_user_message,
};
use xai_grok_shell::sampling::types::{
    REASONING_EFFORT_META_KEY, ReasoningEffort, parse_canonical_effort_token,
    reasoning_effort_meta_value,
};
use xai_grok_shell::util::config as cli_config;
use xai_grok_telemetry::startup::PendingStartup;
mod ext_protocol;
mod mcp_init;
mod prompt_ack;
mod reducer;
mod signals;
use ext_protocol::{ExtEvent, handle_ext_notification, reply_headless_ext_method};
use prompt_ack::{abort_unacknowledged_prompt, headless_ack_signal};
use signals::HeadlessSignals;
mod cli;
pub(crate) use cli::parse_comma_list;
pub use cli::{HeadlessPrompt, OutputFormat, parse_json_schema, parse_permission_rules_lenient};
pub(crate) use cli::{ResolvedAgent, resolve_agent_arg};
use cli::{apply_agent_flag, parse_cli_agents, parse_permission_rules_strict};
#[derive(Debug, Clone)]
pub struct HeadlessOptions {
    pub session_id: Option<String>,
    pub resume: Option<String>,
    /// Resume was pinned pre-sandbox; materialization must not re-run title selection.
    pub resume_title_pinned: bool,
    pub cwd: Option<PathBuf>,
    pub yolo: bool,
    pub trust: bool,
    pub output_format: OutputFormat,
    /// Emit `stream_event` deltas for `streaming-messages-json`.
    pub include_partial_messages: bool,
    pub json_schema: Option<serde_json::Value>,
    pub model: Option<String>,
    pub rules: Option<String>,
    pub system_prompt_override: Option<String>,
    pub continue_last_session: bool,
    /// Fork on resume/continue (`--fork-session`).
    pub fork_session: bool,
    /// `-w`; `Some("")` is a bare, unlabeled `-w`.
    pub worktree: Option<String>,
    /// `--worktree-ref`; set means a clean checkout of that ref, unset overlays the dirty tree.
    pub worktree_ref: Option<String>,
    pub restore_code: bool,
    pub agent: Option<String>,
    pub agents_json: Option<String>,
    pub cli_tools: Option<String>,
    pub cli_disallowed_tools: Option<String>,
    pub disable_web_search: bool,
    pub allow_rules: Vec<String>,
    pub deny_rules: Vec<String>,
    pub max_turns: Option<u32>,
    pub permission_mode_flag: Option<String>,
    /// Effort token (`--reasoning-effort` / `--effort`); a menu id or canonical level after models load.
    pub reasoning_effort: Option<String>,
    /// Wait for background tasks to report `task_completed` before exiting (default true).
    pub wait_for_background: bool,
    /// Max time to wait for background work to finish after the first turn ends.
    pub background_wait_timeout: Duration,
    /// After the prompt (or instead of one when resuming), run `x.ai/memory/flush`.
    pub memory_flush: bool,
    /// CLI `--experimental-memory` / `--no-memory` override for the headless agent.
    pub memory_enabled_override: Option<bool>,
    /// `--no-subagents`. Only the explicit flag reaches the CLI tier; otherwise env and config decide.
    pub no_subagents: bool,
}
/// Apply the headless launch flags to a parsed config. `--no-subagents` uses the same CLI tier as the TUI.
pub(crate) fn apply_headless_runtime(
    agent_config: &mut AgentConfig,
    raw_config: &toml::Value,
    no_subagents: bool,
    memory_enabled_override: Option<bool>,
    disable_web_search: bool,
) {
    agent_config.resolve_runtime_fields(&xai_grok_shell::agent::config::RuntimeResolutionContext {
        raw_config,
        remote_settings: None,
        is_headless: true,
        cli_subagents: no_subagents.then_some(false),
        cli_web_search_model: None,
        cli_session_summary_model: None,
        memory_enabled_override,
        disable_web_search,
        todo_gate: false,
        laziness_debug_log: None,
        storage_mode: None,
    });
}
struct HeadlessEmitter {
    format: OutputFormat,
    parse_structured_output: bool,
    text_buffer: String,
    thought_buffer: String,
    /// Schema-validated output read from the prompt-response `_meta`.
    structured_output: Option<Result<serde_json::Value, String>>,
    usage: Option<serde_json::Value>,
    /// Worktree root once a `-w` session is open. Name collisions make this
    /// differ from the label the user typed.
    worktree_path: Option<String>,
    /// Reducer for the streaming formats; `None` for `plain`/`json`.
    reducer: Option<Box<dyn Reducer>>,
    /// Set when the prompt is sent; `result.duration_ms` on the terminal line is measured from it.
    prompt_started: Option<Instant>,
    out: std::io::Stdout,
    /// Latched once stdout is unwritable so later writes are dropped instead of panicking.
    output_closed: bool,
    /// First hard stdout IO error (not a broken pipe), surfaced so the process exits non-zero.
    write_error: Option<std::io::Error>,
}
impl HeadlessEmitter {
    fn new(format: OutputFormat, parse_structured_output: bool) -> Self {
        Self {
            format,
            parse_structured_output,
            text_buffer: String::new(),
            thought_buffer: String::new(),
            structured_output: None,
            usage: None,
            worktree_path: None,
            reducer: reducer_for(format),
            prompt_started: None,
            out: std::io::stdout(),
            output_closed: false,
            write_error: None,
        }
    }
    /// Checked write to stdout: broken pipe latches a clean stop, any other error is latched and returned.
    fn write_out(&mut self, bytes: &[u8], flush: bool) -> std::io::Result<()> {
        if self.output_closed {
            return Ok(());
        }
        use std::io::Write as _;
        let result = {
            let mut handle = self.out.lock();
            handle
                .write_all(bytes)
                .and_then(|()| if flush { handle.flush() } else { Ok(()) })
        };
        self.record_write_result(result)
    }
    /// Fold a write result into the latches: broken pipe is a clean stop, any other error is surfaced.
    fn record_write_result(&mut self, result: std::io::Result<()>) -> std::io::Result<()> {
        let Err(e) = result else {
            return Ok(());
        };
        self.output_closed = true;
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            tracing::debug!("headless: stdout closed (broken pipe); halting output");
            return Ok(());
        }
        tracing::error!(error = %e, "headless: stdout write failed; halting output");
        if self.write_error.is_none() {
            self.write_error = Some(std::io::Error::new(e.kind(), e.to_string()));
        }
        Err(e)
    }
    /// Take the latched hard stdout error, if any.
    fn take_output_error(&mut self) -> Option<std::io::Error> {
        self.write_error.take()
    }
    /// Emit one compact NDJSON wire line plus newline.
    fn emit_line(&mut self, line: &serde_json::Value) {
        let mut buf = line.to_string();
        buf.push('\n');
        let _ = self.write_out(buf.as_bytes(), false);
    }
    /// Mark the wall-clock start of the run for `result.duration_ms`.
    fn mark_prompt_started(&mut self) {
        self.prompt_started = Some(Instant::now());
    }
    fn duration_ms(&self) -> u64 {
        self.prompt_started
            .map_or(0, |t| t.elapsed().as_millis() as u64)
    }
    /// Emit the reducer preamble once the session context is known.
    fn begin_session(&mut self, ctx: SessionContext) {
        let Some(reducer) = self.reducer.as_mut() else {
            return;
        };
        let lines = reducer.begin(ctx);
        self.emit_lines(lines);
    }
    /// Emit a batch of NDJSON wire lines produced by the reducer.
    fn emit_lines(&mut self, lines: Vec<serde_json::Value>) {
        for line in lines {
            self.emit_line(&line);
        }
    }
    /// Render an `x.ai/*` lifecycle notification for the active format.
    fn on_lifecycle(&mut self, event: Lifecycle) {
        match self.format {
            OutputFormat::Plain => eprint_line(&event.plain_message()),
            OutputFormat::Json => {}
            OutputFormat::StreamingJson | OutputFormat::StreamingMessagesJson => {
                self.reduce_and_emit(StreamEvent::Lifecycle(event));
            }
        }
    }
    /// Fold one event through the reducer and emit its lines; a no-op for `plain`/`json`.
    fn reduce_and_emit(&mut self, event: StreamEvent) {
        let Some(reducer) = self.reducer.as_mut() else {
            return;
        };
        let lines = reducer.reduce(event);
        self.emit_lines(lines);
    }
    /// Schema output for a terminal line: `Ok`, `Err`, or `None` when not requested.
    fn resolved_structured_output(&self) -> Option<Result<serde_json::Value, String>> {
        if !self.parse_structured_output {
            return None;
        }
        Some(
            self.structured_output
                .clone()
                .unwrap_or_else(|| Err("model did not produce structured output".to_string())),
        )
    }
    /// Read structured output (or its error) from the prompt-response `_meta`.
    fn set_structured_output_from_meta(&mut self, meta: Option<&acp::Meta>) {
        if !self.parse_structured_output {
            return;
        }
        let Some(meta) = meta else { return };
        if let Some(err) = meta.get("structuredOutputError").and_then(|v| v.as_str()) {
            self.structured_output = Some(Err(err.to_string()));
        } else if let Some(value) = meta.get("structuredOutput") {
            self.structured_output = Some(Ok(value.clone()));
        }
    }
    fn set_usage_from_meta(&mut self, meta: Option<&acp::Meta>) {
        let Some(meta) = meta else { return };
        self.usage = meta.get("usage").cloned();
    }
    fn on_text_chunk(&mut self, text: &str) {
        match self.format {
            OutputFormat::Plain => {
                let _ = self.write_out(text.as_bytes(), true);
            }
            OutputFormat::Json => {
                self.text_buffer.push_str(text);
            }
            OutputFormat::StreamingMessagesJson => {
                self.text_buffer.push_str(text);
                self.reduce_and_emit(StreamEvent::AgentMessage(text.to_string()));
            }
            OutputFormat::StreamingJson => {
                self.reduce_and_emit(StreamEvent::AgentMessage(text.to_string()));
            }
        }
    }
    fn on_thought_chunk(&mut self, text: &str) {
        match self.format {
            OutputFormat::Plain => {}
            OutputFormat::Json => {
                self.thought_buffer.push_str(text);
            }
            OutputFormat::StreamingJson | OutputFormat::StreamingMessagesJson => {
                self.reduce_and_emit(StreamEvent::AgentThought(text.to_string()));
            }
        }
    }
    fn attach_structured_output(&self, target: &mut serde_json::Value) {
        if !self.parse_structured_output {
            return;
        }
        let result = self
            .structured_output
            .clone()
            .unwrap_or_else(|| Err("model did not produce structured output".to_string()));
        crate::headless::reducer::attach_structured_output(target, Some(result));
    }
    /// Final object for `--output-format json`, including spend fields when present.
    fn build_json_result(
        &self,
        stop_reason: &str,
        session_id: &str,
        request_id: &str,
    ) -> serde_json::Value {
        let mut result = serde_json::json!({
            "text": self.text_buffer,
            "stopReason": stop_reason,
            "sessionId": session_id,
            "requestId": request_id
        });
        if !self.thought_buffer.is_empty()
            && let Some(obj) = result.as_object_mut()
        {
            obj.insert(
                "thought".into(),
                serde_json::Value::String(self.thought_buffer.clone()),
            );
        }
        if let Some(usage) = &self.usage {
            attach_result_usage(&mut result, usage);
        }
        self.attach_worktree_path(&mut result);
        self.attach_structured_output(&mut result);
        result
    }
    /// Remember the worktree folder a started session is using.
    fn note_opened_worktree(&mut self, root: &std::path::Path) {
        self.worktree_path = Some(root.display().to_string());
    }
    fn attach_worktree_path(&self, target: &mut serde_json::Value) {
        let Some(path) = &self.worktree_path else {
            return;
        };
        if let Some(obj) = target.as_object_mut() {
            obj.insert(
                "worktreePath".into(),
                serde_json::Value::String(path.clone()),
            );
        }
    }
    fn reports_single_json_error(&self) -> bool {
        self.format == OutputFormat::Json
    }
    /// One JSON error for a resume, continue, or fork that failed before the session opened.
    /// The returned error is already printed, so the process exit must not print it again.
    fn fail_json_before_open(&mut self, message: &str) -> anyhow::Error {
        self.on_error(message, None);
        anyhow::Error::new(HeadlessFailureAlreadyReported)
    }
    /// A failure whose record is already on stdout.
    /// JSON formats keep the exit status and do not add a second `Error:` line on stderr.
    /// Plain mode still returns `err` so the process can print it.
    fn outcome_error(&self, err: anyhow::Error) -> anyhow::Error {
        if self.format == OutputFormat::Plain {
            err
        } else {
            let _ = err;
            anyhow::Error::new(HeadlessFailureAlreadyReported)
        }
    }
    /// `--output-format json` error object. The worktree folder is present only
    /// after [`Self::note_opened_worktree`].
    fn json_error(&self, message: &str) -> serde_json::Value {
        let mut err = serde_json::json!({"type":"error","message": message});
        if let Some(usage) = &self.usage {
            attach_result_usage(&mut err, usage);
        }
        self.attach_worktree_path(&mut err);
        err
    }
    /// Records a finished turn would write. `plain` has none.
    /// JSON is one object. Streaming formats are the reducer's terminal lines.
    fn finish_turn(
        &mut self,
        stop_reason: &str,
        session_id: &str,
        request_id: &str,
    ) -> Vec<serde_json::Value> {
        match self.format {
            OutputFormat::Plain => Vec::new(),
            OutputFormat::Json => {
                vec![self.build_json_result(stop_reason, session_id, request_id)]
            }
            OutputFormat::StreamingJson | OutputFormat::StreamingMessagesJson => {
                let usage = self.usage.clone();
                let structured_output = self.resolved_structured_output();
                let result_text = self.text_buffer.clone();
                let duration_ms = self.duration_ms();
                self.reducer
                    .as_mut()
                    .map(|reducer| {
                        reducer.finish(&TurnEnd {
                            stop_reason,
                            session_id,
                            request_id,
                            usage: usage.as_ref(),
                            structured_output,
                            result_text: result_text.as_str(),
                            duration_ms,
                        })
                    })
                    .unwrap_or_default()
            }
        }
    }
    fn write_finish_lines(&mut self, lines: &[serde_json::Value]) {
        match self.format {
            OutputFormat::Plain => {
                let _ = self.write_out(b"\n", false);
            }
            OutputFormat::Json => {
                let Some(result) = lines.first() else {
                    return;
                };
                let mut rendered =
                    serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string());
                rendered.push('\n');
                let _ = self.write_out(rendered.as_bytes(), false);
            }
            OutputFormat::StreamingJson | OutputFormat::StreamingMessagesJson => {
                self.emit_lines(lines.to_vec());
            }
        }
    }
    fn on_end(&mut self, stop_reason: &str, session_id: &str, request_id: &str) {
        let lines = self.finish_turn(stop_reason, session_id, request_id);
        self.write_finish_lines(&lines);
    }
    /// Ctrl-C / SIGTERM: write the closing record, then flush, before the process exits.
    /// `stopReason` is `cancelled`. Text and usage gathered so far stay on that record.
    fn close_after_signal(&mut self, session_id: &str) {
        self.on_end("cancelled", session_id, "");
        let _ = self.write_out(b"", true);
    }
    /// Emit the max turns marker for the active format.
    fn on_max_turns(&mut self) {
        match self.format {
            OutputFormat::Plain => eprint_line("Max turns reached"),
            OutputFormat::Json => {}
            OutputFormat::StreamingJson | OutputFormat::StreamingMessagesJson => {
                let lines = self.reducer.as_mut().map(|reducer| reducer.max_turns());
                if let Some(lines) = lines {
                    self.emit_lines(lines);
                }
            }
        }
    }
    /// Emit the terminal error; `stop_reason_override` stamps a Messages stop reason (e.g. `max_tokens`).
    fn on_error(&mut self, message: &str, stop_reason_override: Option<&str>) {
        match self.format {
            OutputFormat::Plain => eprint_line(message),
            OutputFormat::Json => {
                self.emit_line(&self.json_error(message));
            }
            OutputFormat::StreamingJson | OutputFormat::StreamingMessagesJson => {
                let usage = self.usage.clone();
                let duration_ms = self.duration_ms();
                let lines = self.reducer.as_mut().map(|reducer| {
                    reducer.error(message, usage.as_ref(), duration_ms, stop_reason_override)
                });
                if let Some(lines) = lines {
                    self.emit_lines(lines);
                }
            }
        }
    }
}
/// Why a `session/load` failed. An empty ACP error stays the old "Session does not exist".
fn session_load_failure_message(err: &acp::Error) -> String {
    let message = err.message.trim();
    let detail = err
        .data
        .as_ref()
        .and_then(error_detail_from_data)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    match (message.is_empty(), detail) {
        (false, Some(detail)) if detail != message => format!("{message}: {detail}"),
        (false, _) => message.to_string(),
        (true, Some(detail)) => detail,
        (true, None) => "Session does not exist".to_string(),
    }
}
/// Resume, continue, and fork. Continue is a most-recent resume.
fn intent_resumes_or_forks(intent: &crate::app::session_startup::SessionStartupIntent) -> bool {
    use crate::app::session_startup::SessionStartupIntent;
    matches!(
        intent,
        SessionStartupIntent::Resume { .. } | SessionStartupIntent::ForkFrom { .. }
    )
}
/// Marker for a headless failure whose user-facing report was already written.
#[derive(Debug)]
struct HeadlessFailureAlreadyReported;
impl std::fmt::Display for HeadlessFailureAlreadyReported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("headless failure already reported")
    }
}
impl std::error::Error for HeadlessFailureAlreadyReported {}
/// True when headless already printed the failure, so the process must not print it again.
pub fn failure_already_reported(err: &anyhow::Error) -> bool {
    err.chain()
        .any(|cause| cause.is::<HeadlessFailureAlreadyReported>())
}
pub(crate) fn attach_result_usage(result: &mut serde_json::Value, usage: &serde_json::Value) {
    xai_grok_shell::extensions::notification::attach_result_usage_fail_closed(result, usage);
}
/// Snake_case wire token for an ACP stop reason.
fn stop_reason_wire(reason: acp::StopReason) -> String {
    match reason {
        acp::StopReason::EndTurn => "end_turn",
        acp::StopReason::MaxTokens => "max_tokens",
        acp::StopReason::MaxTurnRequests => "max_turn_requests",
        acp::StopReason::Refusal => "refusal",
        acp::StopReason::Cancelled => "cancelled",
        other => {
            tracing::warn!(
                stop_reason = ?other,
                "headless: unknown ACP StopReason; defaulting wire token to end_turn"
            );
            "end_turn"
        }
    }
    .to_string()
}
/// Stop reason written into headless JSON.
/// A turn limit arrives as `Cancelled` plus the max-turns category. JSON names that
/// outcome `max_turn_requests`, the ACP token for hitting the turn limit.
fn headless_result_stop_reason(reason: acp::StopReason, is_max_turns: bool) -> String {
    if is_max_turns {
        "max_turn_requests".to_string()
    } else {
        stop_reason_wire(reason)
    }
}
/// Sentence for a headless JSON error.
/// ACP `Display` prefixes the JSON-RPC label and then pretty-prints `data`, so a
/// detail that already states the failure is printed twice. The detail is the
/// message. A repeated leading `Error:` is dropped.
fn headless_error_message(err: &acp::Error) -> String {
    let owned = err
        .data
        .as_ref()
        .and_then(error_detail_from_data)
        .filter(|detail| !detail.trim().is_empty())
        .unwrap_or_else(|| err.to_string());
    strip_duplicate_error_prefix(owned.trim()).to_string()
}
/// Drop an outer error label when the remainder repeats one.
/// `Error: Error: file is missing` stays `Error: file is missing`.
/// `Internal error: "Error: file is missing"` stays `Error: file is missing`.
fn strip_duplicate_error_prefix(message: &str) -> &str {
    let mut rest = message.trim();
    loop {
        let Some(after_label) = strip_one_error_label(rest) else {
            break;
        };
        let inner = unquote_json_string(after_label.trim_start());
        if strip_one_error_label(inner).is_some() {
            rest = inner;
            continue;
        }
        break;
    }
    rest
}
fn strip_one_error_label(message: &str) -> Option<&str> {
    const LABELS: [&str; 2] = ["internal error:", "error:"];
    let bytes = message.as_bytes();
    for label in LABELS {
        let width = label.len();
        if let Some(head) = bytes.get(..width)
            && head.eq_ignore_ascii_case(label.as_bytes())
        {
            return message.get(width..);
        }
    }
    None
}
fn unquote_json_string(message: &str) -> &str {
    let message = message.trim();
    if message.len() >= 2
        && message.as_bytes().first() == Some(&b'"')
        && message.as_bytes().last() == Some(&b'"')
    {
        message.get(1..message.len() - 1).unwrap_or(message)
    } else {
        message
    }
}
/// Line printed when headless refuses a permission prompt instead of asking.
pub(crate) fn refused_approval_line(req: &acp::RequestPermissionRequest) -> String {
    let title = req
        .tool_call
        .fields
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty());
    match title {
        Some(title) => format!("Refused approval request: {title}"),
        None => "Refused approval request.".to_string(),
    }
}

fn auto_respond_to_permissions(
    args: &acp::RequestPermissionRequest,
    option_kinds: &[acp::PermissionOptionKind],
) -> Option<acp::RequestPermissionResponse> {
    for &option_kind in option_kinds {
        for option in &args.options {
            if option.kind == option_kind {
                return Some(acp::RequestPermissionResponse::new(
                    acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                        option.option_id.clone(),
                    )),
                ));
            }
        }
    }
    None
}
/// "Not signed in" error message, tailored to the session type.
fn auth_required_message(interactive: bool) -> String {
    if interactive {
        "Not signed in. Run `grok login` to authenticate \
         (or `grok login --device-code` if no browser is available)."
            .to_string()
    } else {
        "Not signed in. To authenticate without a browser, run:\n  \
         grok login --device-code\n\n\
         Alternatively, set the XAI_API_KEY environment variable \
         or run `grok login` on a machine with a browser."
            .to_string()
    }
}
/// The same backend switch the TUI applies; the shell unless another backend is enabled.
async fn spawn_agent(
    agent_config: AgentConfig,
    cancel: &CancellationToken,
    memory_config: Option<xai_grok_shell::config::MemoryConfig>,
    options: &HeadlessOptions,
) -> Result<SpawnedAgent> {
    let _ = options;
    spawn_grok_shell(agent_config, cancel, memory_config).await
}
/// `true` when this auth method is the external `auth_provider_command` login.
fn method_runs_auth_provider(method: &acp::AuthMethod) -> bool {
    method
        .meta()
        .and_then(|meta| meta.get("external_provider"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

/// Authenticate via the agent's `defaultAuthMethodId`, failing closed when none is available.
/// Returns whether the selected method is API-key auth.
async fn authenticate(
    acp_tx: &AcpAgentTx,
    auths: &[acp::AuthMethod],
    default_auth_method_id: Option<&acp::AuthMethodId>,
) -> anyhow::Result<bool> {
    let method_id = crate::acp::select_eager_auth_method(auths, default_auth_method_id)
        .ok_or_else(|| {
            use std::io::IsTerminal;
            let interactive = std::io::stdin().is_terminal()
                && !xai_grok_shell::util::clipboard::is_remote_session();
            anyhow::anyhow!("{}", auth_required_message(interactive))
        })?;
    let kind = AuthMethodKind::from_id(&method_id);
    // `grok.com` is interactive for a browser login. With `auth_provider_command` the same
    // method carries `external_provider`, and headless must send it so the shell runs that command
    // when no login is stored.
    let external_provider = auths
        .iter()
        .find(|method| method.id() == &method_id)
        .is_some_and(method_runs_auth_provider);
    if kind.needs_interactive_login() && !external_provider {
        use std::io::IsTerminal;
        let interactive =
            std::io::stdin().is_terminal() && !xai_grok_shell::util::clipboard::is_remote_session();
        anyhow::bail!("{}", auth_required_message(interactive));
    }
    let is_api_key_auth = kind.is_api_key();
    let _resp: acp::AuthenticateResponse = acp_send(
        acp::AuthenticateRequest::new(method_id)
            .meta(serde_json::json!({"headless": true}).as_object().cloned()),
        acp_tx,
    )
    .await?;
    Ok(is_api_key_auth)
}
fn build_headless_init_request(
    rules: Option<&str>,
    system_prompt_override: Option<&str>,
    start_in_plan: bool,
) -> acp::InitializeRequest {
    let mut meta = serde_json::json!({
        "clientType": HEADLESS_CLIENT_TYPE,
        "clientVersion": PAGER_CLIENT_VERSION,
    });
    if let Some(obj) = meta.as_object_mut() {
        if let Some(rules) = rules {
            obj.insert("rules".into(), serde_json::json!(rules));
        }
        if let Some(system_prompt_override) = system_prompt_override {
            obj.insert(
                "systemPromptOverride".into(),
                serde_json::json!(system_prompt_override),
            );
        }
        obj.insert(
            "startupHints".into(),
            serde_json::json!({
                "nonInteractive": true,
                "startInPlan": start_in_plan,
            }),
        );
    }
    acp::InitializeRequest::new(acp::ProtocolVersion::V1)
        .client_capabilities(
            acp::ClientCapabilities::new()
                .fs(acp::FileSystemCapabilities::new())
                .terminal(false),
        )
        .meta(meta.as_object().cloned())
}
#[derive(Debug)]
struct OpenedSession {
    session_id: acp::SessionId,
    models: ModelState,
    /// Directory the session is anchored to (launch cwd, resume `original_cwd`, or fork `write_cwd`).
    cwd: PathBuf,
    /// Set when this session was opened inside a worktree grok just created.
    worktree_root: Option<PathBuf>,
}
async fn open_session(
    acp_tx: &AcpAgentTx,
    cwd: &Path,
    session_id_flag: Option<&str>,
    restore_code: Option<bool>,
) -> anyhow::Result<OpenedSession> {
    let mcp_servers =
        cli_config::load_mcp_servers(cwd, &xai_grok_tools::types::compat::CompatConfig::default());
    if let Some(sid) = session_id_flag {
        let try_load: Result<acp::LoadSessionResponse, _> = acp_send(
            acp::LoadSessionRequest::new(acp::SessionId::new(sid.to_string()), cwd.to_path_buf())
                .mcp_servers(mcp_servers.clone())
                .meta({
                    let mut m = acp::Meta::new();
                    m.insert("noReplay".into(), serde_json::Value::Bool(true));
                    if let Some(rc) = restore_code {
                        m.insert("x.ai/restore_code".into(), serde_json::Value::Bool(rc));
                    }
                    Some(m)
                }),
            acp_tx,
        )
        .await;
        match try_load {
            Ok(resp) => {
                return Ok(OpenedSession {
                    session_id: acp::SessionId::new(sid.to_string()),
                    models: ModelState::from(resp.models),
                    cwd: cwd.to_path_buf(),
                    worktree_root: None,
                });
            }
            Err(err) => anyhow::bail!("{}", session_load_failure_message(&err)),
        }
    }
    let mut meta = serde_json::json!({ "sessionKind": "headless" })
        .as_object()
        .cloned();
    crate::app::session_startup::stamp_phase_traceparent(&mut meta);
    let new_resp: acp::NewSessionResponse = acp_send(
        crate::app::add_dir::attach_startup_dirs(
            acp::NewSessionRequest::new(cwd.to_path_buf())
                .mcp_servers(mcp_servers)
                .meta(meta),
        ),
        acp_tx,
    )
    .await?;
    Ok(OpenedSession {
        session_id: new_resp.session_id,
        models: ModelState::from(new_resp.models),
        cwd: cwd.to_path_buf(),
        worktree_root: None,
    })
}
async fn open_session_with_id(
    acp_tx: &AcpAgentTx,
    cwd: &Path,
    session_id: &str,
) -> anyhow::Result<OpenedSession> {
    let cwd_str = cwd.to_string_lossy();
    crate::app::session_startup::ensure_session_id_available(session_id, &cwd_str)?;
    let mcp_servers =
        cli_config::load_mcp_servers(cwd, &xai_grok_tools::types::compat::CompatConfig::default());
    let mut meta = serde_json::json!({ "sessionId": session_id, "sessionKind": "headless" })
        .as_object()
        .cloned();
    crate::app::session_startup::stamp_phase_traceparent(&mut meta);
    let new_resp: acp::NewSessionResponse = acp_send(
        crate::app::add_dir::attach_startup_dirs(
            acp::NewSessionRequest::new(cwd.to_path_buf())
                .mcp_servers(mcp_servers)
                .meta(meta),
        ),
        acp_tx,
    )
    .await?;
    Ok(OpenedSession {
        session_id: new_resp.session_id,
        models: ModelState::from(new_resp.models),
        cwd: cwd.to_path_buf(),
        worktree_root: None,
    })
}
async fn fork_then_open(
    acp_tx: &AcpAgentTx,
    launch_cwd: &Path,
    parent_id: &str,
    parent_cwd: Option<&Path>,
    new_id: Option<&str>,
    restore_code: Option<bool>,
) -> anyhow::Result<OpenedSession> {
    use crate::app::session_startup::{
        effective_fork_new_cwd, ensure_session_id_available, fork_response_error,
        fork_response_new_session_id, fork_session_params, parent_session_is_worktree,
    };
    let launch_cwd_str = launch_cwd.to_string_lossy().into_owned();
    let new_cwd_str = effective_fork_new_cwd(&launch_cwd_str, parent_cwd);
    let write_cwd = PathBuf::from(&new_cwd_str);
    if let Some(nid) = new_id {
        ensure_session_id_available(nid, &new_cwd_str)?;
    }
    let parent_is_worktree = parent_session_is_worktree(parent_id, &write_cwd);
    let mut payload = fork_session_params(parent_id, &write_cwd, new_id, parent_is_worktree);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "sessionKind".into(),
            serde_json::Value::String("headless".into()),
        );
    }
    let fork_params = serde_json::value::to_raw_value(&payload)
        .map_err(|e| anyhow::anyhow!("serialize fork params: {e}"))?;
    let req = acp::ExtRequest::new("x.ai/session/fork", fork_params.into());
    let resp = acp_send(req, acp_tx).await?;
    if let Some(err) = fork_response_error(resp.0.get()) {
        anyhow::bail!("fork failed: {err}");
    }
    let child = fork_response_new_session_id(resp.0.get())
        .ok_or_else(|| anyhow::anyhow!("fork response missing newSessionId"))?;
    match open_session(acp_tx, &write_cwd, Some(&child), restore_code).await {
        Ok(opened) => Ok(opened),
        Err(e) => Err(anyhow::anyhow!(
            "fork succeeded as {child} but load failed: {e}"
        )),
    }
}
/// Mirrors `Effect::CreateWorktreeSession`. A `-s` UUID also names the worktree, and
/// `open_session_with_id` checks its availability under the worktree cwd, which is why
/// `materialize_startup_for_cwd` skipped that check when `has_worktree` is set.
async fn open_session_in_new_worktree(
    acp_tx: &AcpAgentTx,
    cwd: &Path,
    spec: &WorktreeSpec,
    session_id: Option<&str>,
) -> anyhow::Result<OpenedSession> {
    let created = create_worktree(acp_tx, cwd, spec, &new_worktree_id(session_id))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    tracing::info!(
        worktree = %created.worktree_root.display(),
        session_cwd = %created.session_cwd.display(),
        copy_mode = ?spec.copy_mode(),
        "headless: worktree created"
    );
    let opened = match session_id {
        Some(sid) => open_session_with_id(acp_tx, &created.session_cwd, sid).await,
        None => open_session(acp_tx, &created.session_cwd, None, None).await,
    };
    match opened {
        Ok(mut session) => {
            session.worktree_root = Some(created.worktree_root);
            Ok(session)
        }
        Err(e) => Err(anyhow::anyhow!(
            "{}",
            note_orphaned_worktree(&e.to_string(), &created.worktree_root)
        )),
    }
}
/// Mirrors the `load_session_id` branch of `Effect::CreateWorktreeSession`: the agent creates the
/// worktree and restores into it, then the session is loaded at the cwd it reports.
async fn resume_session_in_new_worktree(
    acp_tx: &AcpAgentTx,
    cwd: &Path,
    spec: &WorktreeSpec,
    session_id: &str,
    restore_code: Option<bool>,
    local_miss: bool,
) -> anyhow::Result<OpenedSession> {
    let resumed = resume_session_into_worktree(
        acp_tx,
        cwd,
        spec,
        session_id,
        restore_code,
        local_miss.then_some(session_id),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    tracing::info!(
        session_id = %resumed.session_id,
        worktree = %resumed.worktree_root.display(),
        session_cwd = %resumed.session_cwd.display(),
        code_restored = resumed.code_restored,
        "headless: session resumed into worktree"
    );
    match open_session(
        acp_tx,
        &resumed.session_cwd,
        Some(&resumed.session_id),
        None,
    )
    .await
    {
        Ok(mut session) => {
            session.worktree_root = Some(resumed.worktree_root);
            Ok(session)
        }
        Err(e) => Err(anyhow::anyhow!(
            "{}",
            note_orphaned_worktree(&e.to_string(), &resumed.worktree_root)
        )),
    }
}
const EFFORT_IGNORED_WARNING: &str =
    "--effort/--reasoning-effort: model does not support reasoning effort; ignoring";

/// Classify `--effort` for `grok -p`.
/// A model that does not support effort is ignored, with [`EFFORT_IGNORED_WARNING`].
/// An unknown level is refused.
fn choose_headless_effort(
    models: &ModelState,
    model_id: &acp::ModelId,
    token: Option<&str>,
) -> Result<(Option<ReasoningEffort>, Option<&'static str>), String> {
    let Some(token) = token else {
        return Ok((None, None));
    };
    if models.available.is_empty() {
        if parse_canonical_effort_token(token).is_none() {
            return Err(format!(
                "--effort/--reasoning-effort: unknown effort level '{token}' \
                 (model catalog unavailable; remapped menu ids require a loaded catalog)"
            ));
        }
        return Ok((None, None));
    }
    match models.resolve_cli_effort_for_model(model_id, token) {
        Ok(effort) => Ok((Some(effort), None)),
        Err(EffortTokenError::Unsupported) => Ok((None, Some(EFFORT_IGNORED_WARNING))),
        Err(err) => Err(format!("--effort/--reasoning-effort: {}", err.message())),
    }
}

/// Apply `-m` / effort after session open.
/// Effort is soft-ignored on a non-supporting model (still applying `-m`) but hard-fails on a genuinely unknown token.
async fn apply_headless_model_and_effort(
    acp_tx: &AcpAgentTx,
    session_id: &acp::SessionId,
    models: &ModelState,
    model_name: Option<&str>,
    effort_token: Option<&str>,
) -> anyhow::Result<()> {
    if model_name.is_none() && effort_token.is_none() {
        return Ok(());
    }
    let model_id = if let Some(name) = model_name {
        models
            .resolve_by_name_or_id(name)
            .unwrap_or_else(|| acp::ModelId::new(name))
    } else {
        match models.current.clone() {
            Some(id) => id,
            None if effort_token
                .is_some_and(|token| parse_canonical_effort_token(token).is_some()) =>
            {
                return Ok(());
            }
            None => {
                anyhow::bail!("--effort/--reasoning-effort: no active model to apply effort to");
            }
        }
    };
    let (effort, warning) = choose_headless_effort(models, &model_id, effort_token)
        .map_err(|message| anyhow::anyhow!(message))?;
    if let Some(warning) = warning {
        tracing::warn!(model = %model_id.0, "{warning}");
        eprint_line(warning);
    }
    if model_name.is_none() && effort.is_none() {
        return Ok(());
    }
    let meta = effort.map(|eff| {
        let mut m = acp::Meta::new();
        m.insert(
            REASONING_EFFORT_META_KEY.to_string(),
            reasoning_effort_meta_value(eff),
        );
        m
    });
    acp_send(
        acp::SetSessionModelRequest::new(session_id.clone(), model_id.clone()).meta(meta),
        acp_tx,
    )
    .await
    .map_err(|e| {
        if let Some(name) = model_name {
            anyhow::anyhow!(
                "Couldn't set model '{}': {}. Run 'grok models' to see available models.",
                name,
                e
            )
        } else {
            anyhow::anyhow!("Couldn't apply reasoning effort: {e}")
        }
    })?;
    tracing::debug!(
        model_id = %model_id.0,
        effort = ?effort,
        "headless: model/effort set"
    );
    Ok(())
}
/// Startup-materialization context for headless (`-p`) runs; never chat mode.
fn headless_materialize_ctx(
    resume_title_pinned: bool,
    restore_code: bool,
    has_worktree: bool,
) -> crate::app::session_startup::MaterializeCtx {
    crate::app::session_startup::MaterializeCtx {
        has_worktree,
        allow_remote_restore:
            crate::app::session_startup::MaterializeCtx::default_allow_remote_restore(),
        chat_mode: false,
        title_resolution: if resume_title_pinned {
            crate::app::session_startup::TitleResolution::PinnedPreSandbox
        } else {
            crate::app::session_startup::TitleResolution::Allowed
        },
        restore_code,
        recent_session_selection: crate::app::session_startup::RecentSessionSelection::Any,
        restore_progress_on_stdout: false,
    }
}
/// Run a headless single-turn prompt: spawn the agent, drive the ACP lifecycle, stream to stdout.
pub async fn run_single_turn(
    prompt: Option<HeadlessPrompt>,
    verbatim: bool,
    options: HeadlessOptions,
) -> Result<()> {
    let signals = HeadlessSignals::install().await?;
    xai_grok_shell::http::set_process_client_mode_headless();
    let cwd = match options.cwd {
        None => std::env::current_dir()?,
        Some(ref p) => dunce::canonicalize(p)?,
    };
    let mut emitter = HeadlessEmitter::new(options.output_format, options.json_schema.is_some());
    if options.include_partial_messages
        && options.output_format != OutputFormat::StreamingMessagesJson
    {
        eprint_line(
            "warning: --include-partial-messages only affects --output-format streaming-messages-json; ignoring it",
        );
    }
    let t_spawn = Instant::now();
    let raw_config = xai_grok_shell::config::load_effective_config()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {e}"))?;
    let mut agent_config = AgentConfig::new_from_toml_cfg(&raw_config)
        .map_err(|e| anyhow::anyhow!("Failed to create agent config: {e}"))?;
    if let Some(ref token) = options.reasoning_effort
        && let Some(effort) = parse_canonical_effort_token(token)
    {
        agent_config.reasoning_effort_override = Some(effort);
    }
    if let Some(ref model) = options.model {
        agent_config.default_model_override = Some(model.clone());
    }
    apply_headless_runtime(
        &mut agent_config,
        &raw_config,
        options.no_subagents,
        options.memory_enabled_override,
        options.disable_web_search,
    );
    agent_config.mode = xai_grok_shell::agent::config::AgentMode::Headless;
    agent_config.default_yolo_mode = options.yolo;
    agent_config.default_auto_mode = xai_grok_shell::util::config::effective_auto_for_launch(
        options.yolo,
        options.permission_mode_flag.as_deref(),
        None,
        xai_grok_shell::util::config::PermissionMode::Ask,
    );
    apply_agent_flag(&options.agent, &mut agent_config);
    if let Some(ref json) = options.agents_json {
        agent_config.cli_agents = parse_cli_agents(json)?;
    }
    agent_config.cli_agent_overrides = xai_grok_shell::agent::config::CliAgentOverrides {
        tools: parse_comma_list(options.cli_tools.as_deref()),
        disallowed_tools: parse_comma_list(options.cli_disallowed_tools.as_deref()),
        permission_rules: parse_permission_rules_strict(&options.allow_rules, &options.deny_rules)?,
        max_turns: options.max_turns,
        permission_mode: options
            .permission_mode_flag
            .as_deref()
            .map(|s| {
                serde_json::from_value(serde_json::Value::String(s.to_string()))
                    .map_err(|e| anyhow::anyhow!("--permission-mode: invalid value: {e}"))
            })
            .transpose()?,
    };
    if options.trust {
        use xai_grok_workspace::folder_trust::{grant_folder_trust, report_cli_trust_grant};
        report_cli_trust_grant(&grant_folder_trust(&cwd));
    }
    let cancel = CancellationToken::new();
    let memory_config = agent_config.memory_config.clone();
    let mut pending_startup = Some(PendingStartup::new());
    let timer = xai_grok_telemetry::startup::begin(crate::acp::Owner::Client);
    let mut report_startup_failure = |timer: &crate::acp::StartupTimer| {
        timer.emit_telemetry(
            crate::acp::AgentKind::Embedded,
            crate::acp::StartupOutcome::Error,
            None,
            false,
        );
        PendingStartup::finish_held(&mut pending_startup, crate::acp::StartupOutcome::Error);
    };
    let spawned = match spawn_agent(agent_config, &cancel, memory_config, &options).await {
        Ok(s) => s,
        Err(e) => {
            report_startup_failure(&timer);
            let msg = format!("Couldn't start session: {e}");
            emitter.on_error(&msg, None);
            return Err(emitter.outcome_error(anyhow::anyhow!("{msg}")));
        }
    };
    let agent_guard = AgentShutdownGuard::new(cancel.clone(), Some(spawned.thread_handle));
    let (acp_tx, mut acp_rx) = (spawned.channel.tx, spawned.channel.rx);
    crate::unified_log::init(acp_tx.clone());
    crate::unified_log::info(
        "pager started",
        None,
        Some(serde_json::json!({"mode": "headless"})),
    );
    crate::unified_log::flush();
    let init_req = build_headless_init_request(
        options.rules.as_deref(),
        options.system_prompt_override.as_deref(),
        options.permission_mode_flag.as_deref() == Some("plan"),
    );
    xai_grok_telemetry::startup::enter(crate::acp::StartupPhase::AcpInitialize);
    let init_resp: acp::InitializeResponse = match acp_send(init_req, &acp_tx).await {
        Ok(r) => r,
        Err(e) => {
            report_startup_failure(&timer);
            let msg = format!("Couldn't initialize: {e}");
            emitter.on_error(&msg, None);
            return Err(emitter.outcome_error(anyhow::anyhow!("{msg}")));
        }
    };
    tracing::debug!(
        elapsed_ms = t_spawn.elapsed().as_millis() as u64,
        "headless: spawn + initialize complete"
    );
    let t_auth = Instant::now();
    xai_grok_telemetry::startup::enter(crate::acp::StartupPhase::EagerAuth);
    let default_auth_method_id = crate::acp::parse_default_auth_method_id(init_resp.meta.as_ref());
    let is_api_key_auth = match authenticate(
        &acp_tx,
        &init_resp.auth_methods,
        default_auth_method_id.as_ref(),
    )
    .await
    {
        Ok(is_api_key) => is_api_key,
        Err(e) => {
            report_startup_failure(&timer);
            emitter.on_error(&e.to_string(), None);
            return Err(emitter.outcome_error(e));
        }
    };
    tracing::debug!(
        elapsed_ms = t_auth.elapsed().as_millis() as u64,
        "headless: authenticate complete"
    );
    timer.emit_telemetry(
        crate::acp::AgentKind::Embedded,
        crate::acp::StartupOutcome::Ok,
        None,
        false,
    );
    use crate::app::session_startup::{self, MaterializedStartup, SessionStartupFlags};
    let has_resume_id = options.resume.as_deref().filter(|s| !s.is_empty());
    let resume_most_recent = options.resume.as_deref() == Some("");
    let worktree =
        WorktreeSpec::from_cli(options.worktree.as_deref(), options.worktree_ref.as_deref());
    let intent = session_startup::session_startup_intent_from_flags(SessionStartupFlags {
        session_id: options.session_id.as_deref(),
        resume_session_id: has_resume_id,
        resume_most_recent,
        continue_last_session: options.continue_last_session,
        fork_session: options.fork_session,
        has_worktree: worktree.is_some(),
    })
    .map_err(|e| anyhow::anyhow!("{e}"))
    .inspect_err(|_| {
        PendingStartup::finish_held(&mut pending_startup, crate::acp::StartupOutcome::Error);
    })?;
    let cwd_str = cwd.to_string_lossy().to_string();
    let materialized = match session_startup::materialize_startup_for_cwd(
        headless_materialize_ctx(
            options.resume_title_pinned,
            options.restore_code,
            worktree.is_some(),
        ),
        intent.clone(),
        &cwd_str,
    )
    .await
    {
        Ok(materialized) => materialized,
        Err(err) => {
            PendingStartup::finish_held(&mut pending_startup, crate::acp::StartupOutcome::Error);
            if intent_resumes_or_forks(&intent) && emitter.reports_single_json_error() {
                return Err(emitter.fail_json_before_open(&err.to_string()));
            }
            return Err(err);
        }
    };
    let restore_code = match &materialized {
        MaterializedStartup::Resume {
            suppress_code_restore: true,
            ..
        }
        | MaterializedStartup::Fork {
            suppress_code_restore: true,
            ..
        } => Some(false),
        _ => options.restore_code.then_some(true),
    };
    let resume_or_fork = matches!(
        &materialized,
        MaterializedStartup::Resume { .. } | MaterializedStartup::Fork { .. }
    );
    let t_session = Instant::now();
    xai_grok_telemetry::startup::enter(crate::acp::StartupPhase::SessionCreate);
    let opened = match (materialized, worktree.as_ref()) {
        (MaterializedStartup::NewAuto, Some(spec)) => {
            open_session_in_new_worktree(&acp_tx, &cwd, spec, None).await
        }
        (MaterializedStartup::NewWithId { session_id }, Some(spec)) => {
            open_session_in_new_worktree(&acp_tx, &cwd, spec, Some(&session_id)).await
        }
        (
            MaterializedStartup::Resume {
                session_id,
                deferred_local_miss,
                ..
            },
            Some(spec),
        ) => {
            resume_session_in_new_worktree(
                &acp_tx,
                &cwd,
                spec,
                &session_id,
                restore_code,
                deferred_local_miss,
            )
            .await
        }
        (MaterializedStartup::NewAuto, None) => open_session(&acp_tx, &cwd, None, None).await,
        (MaterializedStartup::NewWithId { session_id }, None) => {
            open_session_with_id(&acp_tx, &cwd, &session_id).await
        }
        (
            MaterializedStartup::Resume {
                session_id,
                original_cwd,
                ..
            },
            None,
        ) => {
            let load_cwd = original_cwd.as_deref().unwrap_or(cwd.as_path());
            open_session(&acp_tx, load_cwd, Some(session_id.as_str()), restore_code).await
        }
        (
            MaterializedStartup::Fork {
                parent_session_id,
                parent_cwd,
                new_session_id,
                ..
            },
            _,
        ) => {
            fork_then_open(
                &acp_tx,
                &cwd,
                &parent_session_id,
                parent_cwd.as_deref(),
                new_session_id.as_deref(),
                restore_code,
            )
            .await
        }
    };
    let OpenedSession {
        session_id,
        models: session_models,
        cwd: session_cwd,
        worktree_root,
    } = match opened {
        Ok(v) => v,
        Err(e) => {
            PendingStartup::finish_held(&mut pending_startup, crate::acp::StartupOutcome::Error);
            let msg = format!("Couldn't create session: {e}");
            if resume_or_fork && emitter.reports_single_json_error() {
                return Err(emitter.fail_json_before_open(&msg));
            }
            emitter.on_error(&msg, None);
            return Err(emitter.outcome_error(anyhow::anyhow!("{msg}")));
        }
    };
    PendingStartup::finish_held(&mut pending_startup, crate::acp::StartupOutcome::Ok);
    if let Some(root) = worktree_root.as_deref() {
        emitter.note_opened_worktree(root);
    }
    tracing::debug!(
        elapsed_ms = t_session.elapsed().as_millis() as u64,
        session_id = %session_id.0,
        "headless: open_session complete"
    );
    let track_active = std::env::var("GROK_TRACK_HEADLESS").is_ok();
    if track_active {
        let _ = xai_grok_active_sessions::register(xai_grok_active_sessions::ActiveSession {
            session_id: session_id.clone(),
            pid: std::process::id(),
            cwd: cwd.display().to_string(),
            opened_at: chrono::Utc::now(),
        });
    }
    {
        let mcp_servers = if options.output_format == OutputFormat::StreamingMessagesJson {
            mcp_init::resolve_mcp_servers_for_init(&acp_tx, &session_id, &session_cwd).await
        } else {
            Vec::new()
        };
        let model = options
            .model
            .clone()
            .or_else(|| session_models.current_model_id_str().map(str::to_string));
        let permission_mode = options
            .permission_mode_flag
            .clone()
            .or_else(|| options.yolo.then(|| "bypassPermissions".to_string()));
        emitter.begin_session(SessionContext {
            session_id: session_id.0.to_string(),
            model,
            cwd: session_cwd.to_string_lossy().to_string(),
            permission_mode,
            mcp_servers,
            include_partial_messages: options.include_partial_messages,
            api_key_auth: is_api_key_auth,
            context_window: session_models.get_context_window(),
        });
    }
    let effort_unresolved = |token: &str| {
        if parse_canonical_effort_token(token).is_some() {
            return false;
        }
        let target = options
            .model
            .as_deref()
            .and_then(|m| session_models.resolve_by_name_or_id(m))
            .or_else(|| session_models.current.clone());
        match target {
            Some(model_id) => {
                matches!(
                    session_models.resolve_cli_effort_for_model(&model_id, token),
                    Err(EffortTokenError::UnknownToken { .. } | EffortTokenError::NoActiveModel)
                )
            }
            None => true,
        }
    };
    let needs_fresh_catalog = options
        .model
        .as_deref()
        .is_some_and(|m| session_models.resolve_by_name_or_id(m).is_none())
        || options
            .reasoning_effort
            .as_deref()
            .is_some_and(effort_unresolved);
    let session_models = if needs_fresh_catalog {
        match xai_grok_shell::cli_models::fetch_model_state(&acp_tx).await {
            Ok(state) => ModelState::from(Some(state)),
            Err(e) => {
                tracing::warn!(error = %e, "headless: model catalog refresh failed; using session state");
                session_models
            }
        }
    } else {
        session_models
    };
    if let Err(e) = apply_headless_model_and_effort(
        &acp_tx,
        &session_id,
        &session_models,
        options.model.as_deref(),
        options.reasoning_effort.as_deref(),
    )
    .await
    {
        let msg = e.to_string();
        emitter.on_error(&msg, None);
        return Err(emitter.outcome_error(anyhow::anyhow!("{msg}")));
    }
    let t_prompt = Instant::now();
    emitter.mark_prompt_started();
    let mut ttf_logged = false;
    let ack_deadlines = PromptAckDeadlines::from_process_env();
    let (prompt_fut, mut prompt_ack) = match prompt {
        Some(prompt) => {
            let prompt_blocks = prompt.into_content_blocks();
            let mut meta = serde_json::Map::new();
            if verbatim {
                meta.insert("verbatim".to_string(), serde_json::Value::Bool(true));
            }
            if let Some(ref schema) = options.json_schema {
                meta.insert("outputSchema".to_string(), schema.clone());
            }
            meta.insert(
                "screenMode".to_string(),
                serde_json::Value::String("headless".to_string()),
            );
            let prompt_id = uuid::Uuid::new_v4().to_string();
            meta.insert(
                "promptId".to_string(),
                serde_json::Value::String(prompt_id.clone()),
            );
            let request =
                acp::PromptRequest::new(session_id.clone(), prompt_blocks).meta(Some(meta));
            (
                Some(Box::pin(acp_send(request, &acp_tx))),
                Some(PromptAckWatch::new(prompt_id, Instant::now())),
            )
        }
        None => (None, None),
    };
    let mut prompt_result = None;
    let mut pending_bg: HashSet<BackgroundWork> = HashSet::new();
    let mut background_wait_timed_out = false;
    let mut background_lifecycle = BackgroundLifecycleState::default();
    let mut prompt_done_at: Option<Instant> = None;
    let mut connection_closed = false;
    let mut prompt_unacknowledged = false;
    let mut terminated_by_signal = None;
    if let Some(mut prompt_fut) = prompt_fut {
        let deferred_exit = signals.defer_exit();
        loop {
            if emitter.write_error.is_some() {
                tracing::warn!("headless: stdout write failed; stopping the stream loop");
                break;
            }
            if options.wait_for_background && prompt_result.is_some() && pending_bg.is_empty() {
                drain_pending_acp_messages(
                    &mut acp_rx,
                    &mut emitter,
                    t_prompt,
                    &mut ttf_logged,
                    options.yolo,
                    &mut pending_bg,
                    &mut background_lifecycle,
                );
                if pending_bg.is_empty() {
                    tracing::debug!("headless: no pending background tasks, exiting");
                    break;
                }
            }
            if options.wait_for_background
                && let Some(done_at) = prompt_done_at
                && done_at.elapsed() >= options.background_wait_timeout
                && !pending_bg.is_empty()
            {
                tracing::warn!(
                    pending_bg = pending_bg.len(),
                    timeout_secs = options.background_wait_timeout.as_secs(),
                    "headless: background wait timed out, exiting"
                );
                background_wait_timed_out = true;
                break;
            }
            let timeout_deadline = if options.wait_for_background
                && prompt_result.is_some()
                && !pending_bg.is_empty()
                && let Some(done_at) = prompt_done_at
            {
                let remaining = options
                    .background_wait_timeout
                    .saturating_sub(done_at.elapsed());
                if remaining.is_zero() {
                    Duration::from_millis(50)
                } else {
                    remaining
                }
            } else {
                Duration::from_secs(3600)
            };
            let ack_deadline = match prompt_ack.as_ref() {
                Some(watch) => tokio::time::Instant::from_std(watch.hard_deadline(&ack_deadlines)),
                None => tokio::time::Instant::now() + Duration::from_secs(3600),
            };
            tokio::select! {
                biased;
                // First in the biased order so an ACP firehose cannot starve the signal
                () = deferred_exit.signalled() => break,
                msg = acp_rx.recv() => {
                    let Some(msg) = msg else {
                        emitter.on_error("Connection closed unexpectedly", None);
                        connection_closed = true;
                        break;
                    };
                    let msg = msg.boxed();
                    if let Some(watch) = prompt_ack.as_ref()
                        && headless_ack_signal(&msg, &session_id, watch.prompt_id()).is_some()
                    {
                        prompt_ack = None;
                    }
                    handle_headless_acp_message(
                        msg,
                        &mut emitter,
                        t_prompt,
                        &mut ttf_logged,
                        options.yolo,
                        &mut pending_bg,
                        &mut background_lifecycle,
                    );
                }
                res = &mut prompt_fut, if prompt_result.is_none() => {
                    prompt_ack = None;
                    prompt_result = Some(res);
                    prompt_done_at = Some(Instant::now());
                    if !options.wait_for_background {
                        drain_acp_with_grace(
                            &mut acp_rx,
                            Duration::from_millis(750),
                            &mut emitter,
                            t_prompt,
                            &mut ttf_logged,
                            options.yolo,
                            &mut pending_bg,
                            &mut background_lifecycle,
                        )
                        .await;
                        break;
                    }
                    // Drain now so a task_backgrounded around completion is recorded before the empty-check.
                    drain_pending_acp_messages(
                        &mut acp_rx,
                        &mut emitter,
                        t_prompt,
                        &mut ttf_logged,
                        options.yolo,
                        &mut pending_bg,
                        &mut background_lifecycle,
                    );
                }
                _ = tokio::time::sleep(timeout_deadline), if options.wait_for_background
                    && prompt_result.is_some()
                    && !pending_bg.is_empty() =>
                {
                    // Wake to re-check the timeout at the top of the loop.
                }
                _ = tokio::time::sleep_until(ack_deadline), if prompt_ack.is_some() => {
                    let Some(watch) = prompt_ack.take() else {
                        unreachable!("branch precondition is `prompt_ack.is_some()`")
                    };
                    let err = abort_unacknowledged_prompt(
                        &acp_tx,
                        &session_id,
                        watch.prompt_id(),
                        watch.waited(Instant::now()),
                        &ack_deadlines,
                    )
                    .await;
                    prompt_result = Some(Err(err));
                    prompt_unacknowledged = true;
                    break;
                }
            }
        }
        terminated_by_signal = deferred_exit.release();
        drain_pending_acp_messages(
            &mut acp_rx,
            &mut emitter,
            t_prompt,
            &mut ttf_logged,
            options.yolo,
            &mut pending_bg,
            &mut background_lifecycle,
        );
        if !pending_bg.is_empty() {
            tracing::warn!(
                pending_bg = pending_bg.len(),
                "headless: killing background work still pending at exit"
            );
            reap_pending_background_tasks(
                &pending_bg,
                &background_lifecycle.finished_goals,
                &session_id,
                &acp_tx,
            )
            .await;
        }
    }
    flush_unified_log_at_exit(
        crate::unified_log::flush_blocking(),
        prompt_unacknowledged,
        terminated_by_signal,
    )
    .await;
    if track_active {
        let _ = xai_grok_active_sessions::try_unregister(&session_id);
    }
    if let Some(code) = terminated_by_signal {
        tracing::info!(
            exit_code = code,
            "headless: a signal during the turn ends the run"
        );
        emitter.close_after_signal(session_id.0.as_ref());
        drop(agent_guard);
        crate::app::signal_handler::force_exit(code);
    }
    let mut outcome: Result<()> = match prompt_result {
        _ if connection_closed => Err(anyhow::anyhow!("Connection closed unexpectedly")),
        Some(Ok(resp)) => {
            let is_max_turns = resp
                .meta
                .as_ref()
                .and_then(|m| m.get(crate::app::CANCELLATION_CATEGORY_KEY))
                .and_then(|v| v.as_str())
                == Some(xai_grok_shell::session::commands::MAX_TURNS_REACHED_CATEGORY);
            let stop_reason = headless_result_stop_reason(resp.stop_reason, is_max_turns);
            emitter.set_structured_output_from_meta(resp.meta.as_ref());
            emitter.set_usage_from_meta(resp.meta.as_ref());
            let sid = resp
                .meta
                .as_ref()
                .and_then(|m| m.get("sessionId"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| session_id.0.as_ref());
            let rid = match resp
                .meta
                .as_ref()
                .and_then(|m| m.get("requestId"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                Some(r) => r,
                None => {
                    tracing::warn!(
                        "headless: prompt response carried no requestId; emitting an empty requestId"
                    );
                    ""
                }
            };
            if is_max_turns {
                emitter.on_max_turns();
                emitter.on_end(&stop_reason, sid, rid);
                Err(emitter.outcome_error(anyhow::anyhow!("max turns reached")))
            } else {
                emitter.on_end(&stop_reason, sid, rid);
                Ok(())
            }
        }
        Some(Err(err)) => {
            let msg = if i32::from(err.code) == RATE_LIMITED_ERROR_CODE {
                let detail = err.data.as_ref().and_then(error_detail_from_data);
                let sanitized = crate::app::sanitize_user_error(&format_rate_limited_user_message(
                    detail.as_deref(),
                    is_api_key_auth,
                ));
                strip_duplicate_error_prefix(&sanitized).to_string()
            } else {
                headless_error_message(&err)
            };
            if let Some(usage) = xai_grok_shell::sampling::error::prompt_usage_from_error(&err) {
                match serde_json::to_value(&usage) {
                    Ok(v) => emitter.usage = Some(v),
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "headless: failed to serialize prompt-error usage; spend fields dropped"
                        )
                    }
                }
            }
            let stop_reason_override =
                (xai_grok_shell::sampling::error::stop_reason_for_turn_error(&err) == "MaxTokens")
                    .then_some("max_tokens");
            emitter.on_error(&msg, stop_reason_override);
            Err(emitter.outcome_error(anyhow::anyhow!("{msg}")))
        }
        None => Ok(()),
    };
    outcome = apply_background_wait_timeout(outcome, background_wait_timed_out);
    let flush_error = if options.memory_flush && outcome.is_ok() {
        run_headless_memory_flush(
            &acp_tx,
            &mut acp_rx,
            &session_id,
            &mut emitter,
            options.yolo,
            &mut background_lifecycle,
        )
        .await
        .err()
    } else {
        None
    };
    xai_grok_shell::upload::drain_pending_uploads_at_exit().await;
    if let Some(err) = emitter.take_output_error() {
        return Err(anyhow::Error::new(err).context("headless: stdout write failed"));
    }
    if let Some(e) = flush_error {
        return Err(e);
    }
    outcome
}
/// Invoke `x.ai/memory/flush` and wait for the flush LLM to finish.
async fn run_headless_memory_flush(
    acp_tx: &AcpAgentTx,
    acp_rx: &mut AcpClientRx,
    session_id: &acp::SessionId,
    emitter: &mut HeadlessEmitter,
    yolo: bool,
    background_lifecycle: &mut BackgroundLifecycleState,
) -> Result<()> {
    let params = serde_json::json!({ "session_id": session_id.0.to_string() });
    let raw = serde_json::value::to_raw_value(&params)
        .map_err(|e| anyhow::anyhow!("serialize memory flush params: {e}"))?;
    let request = acp::ExtRequest::new("x.ai/memory/flush", raw.into());
    let mut flush_fut = Box::pin(acp_send(request, acp_tx));
    let t0 = Instant::now();
    let mut ttf_logged = true;
    let mut pending_bg = HashSet::new();
    let response = loop {
        tokio::select! {
            biased;
            msg = acp_rx.recv() => {
                let Some(msg) = msg else {
                    anyhow::bail!("connection closed while waiting for memory flush");
                };
                handle_headless_acp_message(
                    msg.boxed(),
                    emitter,
                    t0,
                    &mut ttf_logged,
                    yolo,
                    &mut pending_bg,
                    background_lifecycle,
                );
            }
            res = &mut flush_fut => break res,
        }
    };
    drain_pending_acp_messages(
        acp_rx,
        emitter,
        t0,
        &mut ttf_logged,
        yolo,
        &mut pending_bg,
        background_lifecycle,
    );
    let response = response.map_err(|e| anyhow::anyhow!("memory flush failed: {e}"))?;
    let response = serde_json::from_str::<MemoryFlushResponse>(response.0.get())
        .map_err(|e| anyhow::anyhow!("memory flush returned an unreadable response: {e}"))?;
    if !response.flushed {
        anyhow::bail!("memory flush skipped: {}", response.summary());
    }
    Ok(())
}
/// Background work tracked for exit: bash/monitor tasks and background subagents, keyed by id.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum BackgroundWork {
    Task(String),
    Subagent(String),
    Goal(String),
    /// Prompt id of the follow-up turn a finished background task started.
    FollowUp(String),
}

/// An `active` goal is still running. Every other status is an outcome headless can report and stop waiting on.
pub(crate) fn goal_is_still_running(status: &str) -> bool {
    status.eq_ignore_ascii_case("active")
}

pub(crate) fn apply_goal_background_update(
    pending: &mut HashSet<BackgroundWork>,
    goal_id: &str,
    status: &str,
) {
    let work = BackgroundWork::Goal(goal_id.to_string());
    if goal_is_still_running(status) {
        pending.insert(work);
    } else {
        pending.remove(&work);
    }
}

pub(crate) fn format_goal_outcome(status: &str, objective: &str) -> String {
    let status = status.trim();
    let objective = objective.trim();
    if objective.is_empty() {
        format!("Goal {status}.")
    } else {
        format!("Goal {status}: {objective}")
    }
}

fn goal_status_is_success(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "complete" | "completed" | "done"
    )
}

/// Record one `GoalUpdated` and return the status and objective to print.
/// A goal that already reported success stays quiet: a later pause must not print
/// `Goal paused.`, and a stale `active` must not put it back in the exit set.
fn record_headless_goal_update(
    pending: &mut HashSet<BackgroundWork>,
    finished_goals: &mut HashSet<String>,
    goal_id: &str,
    status: &str,
    objective: &str,
) -> Option<(String, String)> {
    let status = status.trim();
    if status.is_empty() {
        return None;
    }
    if finished_goals.contains(goal_id) {
        pending.remove(&BackgroundWork::Goal(goal_id.to_string()));
        return None;
    }
    apply_goal_background_update(pending, goal_id, status);
    if goal_is_still_running(status) {
        return None;
    }
    if goal_status_is_success(status) {
        finished_goals.insert(goal_id.to_string());
    }
    Some((status.to_string(), objective.to_string()))
}

/// Exit sends `/goal pause` only for a goal that has not already finished.
/// That command is what prints "Goal paused. Use /goal resume to continue."
fn headless_exit_pauses_goal(finished_goals: &HashSet<String>, goal_id: &str) -> bool {
    !finished_goals.contains(goal_id)
}

pub(crate) fn background_wait_timeout_message() -> &'static str {
    "background wait timed out before pending work finished"
}

/// A successful first turn is still an error when the background wait gives up.
fn apply_background_wait_timeout(outcome: Result<()>, timed_out: bool) -> Result<()> {
    if timed_out {
        Err(anyhow::anyhow!(background_wait_timeout_message()))
    } else {
        outcome
    }
}

/// Prompt id the shell uses when a finished task starts a follow-up turn.
pub(crate) fn task_follow_up_prompt_id(task_id: &str) -> String {
    format!("task-completed-{task_id}")
}

const GOAL_PAUSE_PROMPT: &str = "/goal pause";
#[derive(Default)]
struct BackgroundLifecycleState {
    completed_tasks: HashSet<String>,
    subagents: HashMap<String, SubagentLifecycleState>,
    /// Goal ids that already reported a success status. Later pause lines stay quiet.
    finished_goals: HashSet<String>,
    /// Follow-up prompt ids that already finished. A late `will_wake` must not wait again.
    finished_follow_ups: HashSet<String>,
}
/// Ext request that kills one unit of background work (subagent cancel or task kill).
fn reap_request_for_work(
    work: &BackgroundWork,
    session_id: &acp::SessionId,
) -> serde_json::Result<acp::ExtRequest> {
    let (method, params) = match work {
        BackgroundWork::Subagent(id) => (
            "x.ai/subagent/cancel",
            serde_json::value::to_raw_value(&CancelSubagentRequest {
                subagent_id: id.clone(),
            })?,
        ),
        BackgroundWork::Task(id) => (
            "x.ai/task/kill",
            serde_json::value::to_raw_value(&KillTaskRequest {
                session_id: session_id.0.to_string(),
                task_id: id.clone(),
                source: xai_grok_shell::extensions::task::TaskKillSource::Teardown,
            })?,
        ),
        BackgroundWork::Goal(_) => {
            return Err(serde::de::Error::custom(
                "an active goal is paused with /goal pause, not an ext request",
            ));
        }
        BackgroundWork::FollowUp(_) => {
            return Err(serde::de::Error::custom(
                "a follow-up turn is not killed with a task reap request",
            ));
        }
    };
    Ok(acp::ExtRequest::new(method, params.into()))
}
/// The flush waits on the shell's reply, which may never come when the shell never took the prompt,
/// or when a signal ends the run and a closed terminal leaves nobody to send the second one.
async fn flush_unified_log_at_exit(
    flush: impl Future<Output = ()>,
    prompt_unacknowledged: bool,
    terminated_by_signal: Option<i32>,
) {
    if !prompt_unacknowledged && terminated_by_signal.is_none() {
        flush.await;
        return;
    }
    if tokio::time::timeout(prompt_ack::HEADLESS_ABORT_SEND_TIMEOUT, flush)
        .await
        .is_err()
    {
        tracing::warn!("headless: unified log flush timed out at exit");
    }
}
/// Best-effort kill of background work still pending at exit so it never outlives the process.
async fn reap_pending_background_tasks(
    pending_bg: &HashSet<BackgroundWork>,
    finished_goals: &HashSet<String>,
    session_id: &acp::SessionId,
    acp_tx: &AcpAgentTx,
) {
    for work in pending_bg {
        if let BackgroundWork::Goal(goal_id) = work {
            if headless_exit_pauses_goal(finished_goals, goal_id) {
                pause_headless_goal(session_id, acp_tx).await;
            }
            continue;
        }
        if matches!(work, BackgroundWork::FollowUp(_)) {
            continue;
        }
        let request = match reap_request_for_work(work, session_id) {
            Ok(request) => request,
            Err(e) => {
                tracing::warn!(?work, error = %e, "headless: failed to build reap request");
                continue;
            }
        };
        let method = request.method.clone();
        match tokio::time::timeout(Duration::from_secs(10), acp_send(request, acp_tx)).await {
            Ok(Ok(_)) => {
                tracing::debug!(?work, %method, "headless: reaped pending background work")
            }
            Ok(Err(e)) => {
                tracing::warn!(?work, %method, error = %e, "headless: failed to reap background work")
            }
            Err(_) => {
                tracing::warn!(?work, %method, "headless: timed out reaping background work")
            }
        }
    }
}
async fn pause_headless_goal(session_id: &acp::SessionId, acp_tx: &AcpAgentTx) {
    let request = acp::PromptRequest::new(
        session_id.clone(),
        vec![acp::ContentBlock::Text(acp::TextContent::new(
            GOAL_PAUSE_PROMPT,
        ))],
    );
    match tokio::time::timeout(Duration::from_secs(10), acp_send(request, acp_tx)).await {
        Ok(Ok(_)) => tracing::debug!("headless: paused the active goal"),
        Ok(Err(error)) => {
            tracing::warn!(error = %error, "headless: failed to pause the active goal")
        }
        Err(_) => tracing::warn!("headless: timed out pausing the active goal"),
    }
}

/// A tool result that already carries a finished task or subagent lets headless stop waiting on it.
/// The completion notification can be missing once that output was read in the same turn.
fn note_terminal_output_read(
    pending: &mut HashSet<BackgroundWork>,
    state: &mut BackgroundLifecycleState,
    raw: Option<&serde_json::Value>,
) {
    let Some(raw) = raw else {
        return;
    };
    for id in terminal_work_ids(raw) {
        release_finished_background(pending, state, &id);
    }
}

fn terminal_work_ids(raw: &serde_json::Value) -> Vec<String> {
    let Some(kind) = raw.get("type").and_then(|value| value.as_str()) else {
        return Vec::new();
    };
    match kind {
        "TaskOutput" => task_output_ids(raw),
        "SubagentCompleted" => raw
            .get("subagent_id")
            .and_then(|value| value.as_str())
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .into_iter()
            .collect(),
        "Text" => raw
            .get("consumed_completion_task_id")
            .and_then(|value| value.as_str())
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn task_output_ids(raw: &serde_json::Value) -> Vec<String> {
    if let Some(result) = raw.get("Result") {
        return terminal_result_id(result).into_iter().collect();
    }
    let Some(results) = raw
        .get("MultiResult")
        .and_then(|multi| multi.get("results"))
        .and_then(|results| results.as_array())
    else {
        return Vec::new();
    };
    results.iter().filter_map(terminal_result_id).collect()
}

fn terminal_result_id(result: &serde_json::Value) -> Option<String> {
    let status = result.get("status").and_then(|value| value.as_str())?;
    if !matches!(status, "completed" | "failed" | "cancelled" | "timed_out") {
        return None;
    }
    result
        .get("task_id")
        .and_then(|value| value.as_str())
        .filter(|id| !id.is_empty())
        .map(str::to_string)
}

fn release_finished_background(
    pending: &mut HashSet<BackgroundWork>,
    state: &mut BackgroundLifecycleState,
    id: &str,
) {
    state.completed_tasks.insert(id.to_string());
    if state.subagents.contains_key(id) {
        track_background_lifecycle(
            ExtEvent::SubagentFinished {
                subagent_id: id.to_string(),
                attempt_id: None,
                event_seq: None,
            },
            pending,
            state,
        );
    }
    pending.remove(&BackgroundWork::Task(id.to_string()));
    pending.remove(&BackgroundWork::Subagent(id.to_string()));
}

fn track_background_lifecycle(
    event: ExtEvent,
    pending_bg: &mut HashSet<BackgroundWork>,
    state: &mut BackgroundLifecycleState,
) {
    match event {
        ExtEvent::TaskBackgrounded {
            task_id,
            is_monitor,
        } => {
            if state.completed_tasks.contains(&task_id) {
                tracing::debug!(
                    is_monitor,
                    "headless: ignoring task_backgrounded for already-completed task"
                );
            } else {
                pending_bg.insert(BackgroundWork::Task(task_id));
            }
        }
        ExtEvent::TaskCompleted { task_id, will_wake } => {
            pending_bg.remove(&BackgroundWork::Task(task_id.clone()));
            state.completed_tasks.insert(task_id.clone());
            if will_wake {
                let prompt_id = task_follow_up_prompt_id(&task_id);
                if !state.finished_follow_ups.contains(&prompt_id) {
                    pending_bg.insert(BackgroundWork::FollowUp(prompt_id));
                }
            }
        }
        ExtEvent::TurnCompleted { prompt_id } => {
            state.finished_follow_ups.insert(prompt_id.clone());
            pending_bg.remove(&BackgroundWork::FollowUp(prompt_id));
        }
        ExtEvent::SubagentSpawned {
            subagent_id,
            attempt_id,
            event_seq,
        } => {
            let lifecycle = state.subagents.entry(subagent_id.clone()).or_default();
            if let SubagentLifecycleReduction::Accepted(accepted) = lifecycle.reduce(
                SubagentLifecycleTransition::Spawned,
                attempt_id.as_deref(),
                event_seq,
            ) {
                let effect = accepted.effect();
                accepted.commit(lifecycle);
                match effect {
                    SubagentLifecycleEffect::Apply | SubagentLifecycleEffect::ApplyNewAttempt => {
                        pending_bg.insert(BackgroundWork::Subagent(subagent_id));
                    }
                    SubagentLifecycleEffect::ApplyWithPendingFinish => {
                        pending_bg.remove(&BackgroundWork::Subagent(subagent_id));
                    }
                    SubagentLifecycleEffect::AwaitSpawn | SubagentLifecycleEffect::RecordOnly => {}
                }
            }
        }
        ExtEvent::SubagentProgress {
            subagent_id,
            attempt_id,
            event_seq,
        } => {
            if let Some(lifecycle) = state.subagents.get_mut(&subagent_id)
                && let SubagentLifecycleReduction::Accepted(accepted) = lifecycle.reduce(
                    SubagentLifecycleTransition::Progress,
                    attempt_id.as_deref(),
                    event_seq,
                )
            {
                accepted.commit(lifecycle);
            }
        }
        ExtEvent::SubagentFinished {
            subagent_id,
            attempt_id,
            event_seq,
        } => {
            let lifecycle = state.subagents.entry(subagent_id.clone()).or_default();
            if let SubagentLifecycleReduction::Accepted(accepted) = lifecycle.reduce(
                SubagentLifecycleTransition::Finished,
                attempt_id.as_deref(),
                event_seq,
            ) {
                let effect = accepted.effect();
                accepted.commit(lifecycle);
                if effect == SubagentLifecycleEffect::Apply {
                    pending_bg.remove(&BackgroundWork::Subagent(subagent_id));
                }
            }
        }
        ExtEvent::GoalUpdated { .. }
        | ExtEvent::MonitorEvent
        | ExtEvent::None
        | ExtEvent::Lifecycle(_)
        | ExtEvent::Stream(_) => {}
    }
}
/// Non-blocking drain-to-empty of `acp_rx`.
/// Background work buffered around prompt completion is recorded in `pending_bg` before the empty-check decides whether to exit.
#[allow(clippy::too_many_arguments)]
fn drain_pending_acp_messages(
    acp_rx: &mut AcpClientRx,
    emitter: &mut HeadlessEmitter,
    t_prompt: Instant,
    ttf_logged: &mut bool,
    yolo: bool,
    pending_bg: &mut HashSet<BackgroundWork>,
    background_lifecycle: &mut BackgroundLifecycleState,
) {
    while let Ok(msg) = acp_rx.try_recv() {
        handle_headless_acp_message(
            msg.boxed(),
            emitter,
            t_prompt,
            ttf_logged,
            yolo,
            pending_bg,
            background_lifecycle,
        );
    }
}
#[allow(clippy::too_many_arguments)]
async fn drain_acp_with_grace(
    acp_rx: &mut AcpClientRx,
    grace: Duration,
    emitter: &mut HeadlessEmitter,
    t_prompt: Instant,
    ttf_logged: &mut bool,
    yolo: bool,
    pending_bg: &mut HashSet<BackgroundWork>,
    background_lifecycle: &mut BackgroundLifecycleState,
) {
    let deadline = Instant::now() + grace;
    loop {
        while let Ok(msg) = acp_rx.try_recv() {
            handle_headless_acp_message(
                msg.boxed(),
                emitter,
                t_prompt,
                ttf_logged,
                yolo,
                pending_bg,
                background_lifecycle,
            );
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        tokio::select! {
            biased;
            msg = acp_rx.recv() => {
                let Some(msg) = msg else { break; };
                handle_headless_acp_message(
                    msg.boxed(),
                    emitter,
                    t_prompt,
                    ttf_logged,
                    yolo,
                    pending_bg,
                    background_lifecycle,
                );
            }
            _ = tokio::time::sleep(remaining) => {
                break;
            }
        }
    }
}
/// Process one inbound ACP client message; shared by `recv()` and `try_recv()`.
#[allow(clippy::too_many_arguments)]
fn handle_headless_acp_message(
    msg: AcpClientMessageBox,
    emitter: &mut HeadlessEmitter,
    t_prompt: Instant,
    ttf_logged: &mut bool,
    yolo: bool,
    pending_bg: &mut HashSet<BackgroundWork>,
    background_lifecycle: &mut BackgroundLifecycleState,
) {
    match msg {
        AcpClientMessageBox::SessionNotification(boxed) => {
            match &boxed.request.update {
                acp::SessionUpdate::AgentMessageChunk(chunk) => {
                    if let acp::ContentBlock::Text(text) = &chunk.content
                        && !text.text.is_empty()
                    {
                        if !*ttf_logged {
                            *ttf_logged = true;
                            tracing::debug!(
                                elapsed_ms = t_prompt.elapsed().as_millis() as u64,
                                "headless: time-to-first-chunk"
                            );
                        }
                        emitter.on_text_chunk(&text.text);
                    }
                }
                acp::SessionUpdate::AgentThoughtChunk(chunk) => {
                    if let acp::ContentBlock::Text(text) = &chunk.content
                        && !text.text.is_empty()
                    {
                        if !*ttf_logged {
                            *ttf_logged = true;
                            tracing::debug!(
                                elapsed_ms = t_prompt.elapsed().as_millis() as u64,
                                "headless: time-to-first-thought"
                            );
                        }
                        emitter.on_thought_chunk(&text.text);
                    }
                }
                acp::SessionUpdate::ToolCall(call) => {
                    note_terminal_output_read(
                        pending_bg,
                        background_lifecycle,
                        call.raw_output.as_ref(),
                    );
                    if let Some(event) = map_session_update(&boxed.request.update) {
                        emitter.reduce_and_emit(event);
                    }
                }
                acp::SessionUpdate::ToolCallUpdate(update) => {
                    note_terminal_output_read(
                        pending_bg,
                        background_lifecycle,
                        update.fields.raw_output.as_ref(),
                    );
                    if let Some(event) = map_session_update(&boxed.request.update) {
                        emitter.reduce_and_emit(event);
                    }
                }
                acp::SessionUpdate::Plan(_)
                | acp::SessionUpdate::AvailableCommandsUpdate(_) => {
                    if let Some(event) = map_session_update(&boxed.request.update) {
                        emitter.reduce_and_emit(event);
                    }
                }
                _ => {}
            }
            let _ = boxed.response_tx.send(Ok(()));
        }
        AcpClientMessageBox::RequestPermission(req) => {
            if yolo {
                if let Some(resp) = auto_respond_to_permissions(
                    &req.request,
                    &[
                        acp::PermissionOptionKind::AllowOnce,
                        acp::PermissionOptionKind::AllowAlways,
                    ],
                ) {
                    let _ = req.response_tx.send(Ok(resp));
                } else {
                    eprint_line(&refused_approval_line(&req.request));
                    let _ = req.response_tx.send(Ok(acp::RequestPermissionResponse::new(
                        acp::RequestPermissionOutcome::Cancelled,
                    )));
                }
            } else {
                eprint_line(&refused_approval_line(&req.request));
                let _ = req.response_tx.send(Ok(acp::RequestPermissionResponse::new(
                    acp::RequestPermissionOutcome::Cancelled,
                )));
            }
        }
        AcpClientMessageBox::ExtNotification(notif) => {
            let event = handle_ext_notification(&notif);
            let _ = notif.response_tx.send(Ok(()));
            match event {
                ExtEvent::Lifecycle(l) => emitter.on_lifecycle(l),
                ExtEvent::Stream(event) => emitter.reduce_and_emit(*event),
                ExtEvent::GoalUpdated {
                    goal_id,
                    status,
                    objective,
                } => {
                    if let Some((status, objective)) = record_headless_goal_update(
                        pending_bg,
                        &mut background_lifecycle.finished_goals,
                        &goal_id,
                        &status,
                        &objective,
                    ) {
                        emitter.on_lifecycle(crate::headless::reducer::Lifecycle::GoalFinished {
                            status,
                            objective,
                        });
                    }
                }
                other => track_background_lifecycle(other, pending_bg, background_lifecycle),
            }
        }
        AcpClientMessageBox::WaitForTerminalExit(args) => {
            args.response_tx
                .send(Err(crate::acp::wait_for_exit_not_supported(
                    "headless mode",
                )))
                .ok();
        }
        AcpClientMessageBox::ExtMethod(args) => reply_headless_ext_method(args),
        _ => {}
    }
}
#[cfg(test)]
#[path = "headless/background_lifecycle_tests.rs"]
mod background_lifecycle_tests;
#[cfg(test)]
#[path = "headless/auth_provider_tests.rs"]
mod auth_provider_tests;
#[cfg(test)]
#[path = "headless_tests.rs"]
mod tests;
