use super::*;

pub(super) fn handle_permission_request(
    perm: xai_acp_lib::AcpArgs<acp::RequestPermissionRequest>,
    app: &mut AppView,
) -> bool {
    let matched = match find_session_match(app, &perm.request.session_id) {
        Some(m) => m,
        None => {
            tracing::warn!(
                session_id = %perm.request.session_id.0,
                "Permission request for unknown session_id; cancelling"
            );
            cancel_permission(perm);
            return false;
        }
    };
    let owning_agent_id = matched.agent_id();
    let is_active = is_matched_agent_active(app, owning_agent_id);
    let Some(agent) = app.agents.get_mut(&owning_agent_id) else {
        cancel_permission(perm);
        return false;
    };

    if agent.session.is_yolo()
        && let Some(allow) = perm
            .request
            .options
            .iter()
            .find(|o| o.kind == acp::PermissionOptionKind::AllowOnce)
    {
        let option_id = allow.option_id.clone();
        perm.response_tx
            .send(Ok(acp::RequestPermissionResponse::new(
                acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                    option_id,
                )),
            )))
            .ok();
        return false;
    }

    if !app
        .notification_service
        .should_suppress_permission_notification()
    {
        app.notification_service.notify(NotificationEvent {
            kind: NotificationEventKind::ApprovalRequired,
            title: "Grok".into(),
            body: NotificationEventKind::ApprovalRequired.as_ref().into(),
            session_id: Some(perm.request.session_id.0.to_string()),
        });
        app.notification_service.mark_permission_notified();
    }

    let needs_redraw = enqueue_permission(perm, agent);
    needs_redraw && is_active
}

fn enqueue_permission(
    perm: xai_acp_lib::AcpArgs<acp::RequestPermissionRequest>,
    agent: &mut AgentView,
) -> bool {
    // An open `/feedback` form keeps the keyboard. The permission card waits underneath.
    let bash_highlights: Option<BashCommandHighlights> = perm
        .request
        .meta
        .as_ref()
        .and_then(|meta| serde_json::from_value(serde_json::Value::Object(meta.clone())).ok());
    let bash_selection_count = bash_highlights
        .as_ref()
        .map(|h| xai_grok_workspace::permission::default_always_allow_scope(&h.highlighted_words))
        .unwrap_or(0);
    let bash_deny_selection_count = bash_highlights
        .as_ref()
        .map(|h| xai_grok_workspace::permission::default_always_deny_scope(&h.highlighted_words))
        .unwrap_or(0);

    if let Some(h) = bash_highlights.as_ref() {
        let offers_allow_row = perm.request.options.iter().any(|o| {
            o.option_id.0.as_ref() == crate::views::permission_view::ALLOW_ALWAYS_COMMAND_OPTION_ID
        });
        if offers_allow_row
            && !xai_grok_workspace::permission::always_allow_scope_persists(h, bash_selection_count)
        {
            tracing::warn!(
                scope = bash_selection_count,
                words = ?h.highlighted_words,
                "allow-always-command row offered but default scope does not persist"
            );
        }
    }

    let mcp_scope = perm
        .request
        .options
        .iter()
        .find(|o| o.option_id.0.as_ref() == "allow-always-mcp")
        .and_then(|opt| opt.meta.as_ref())
        .and_then(|m| {
            serde_json::from_value::<xai_grok_workspace::permission::McpToolPermission>(
                serde_json::Value::Object(m.clone()),
            )
            .ok()
        })
        .map(|perm| McpScopeState {
            tool_name: perm.tool_name,
            server_prefix: perm.server_prefix,
            selected: McpScope::Tool,
        });

    let subagent_label = resolve_subagent_label(agent, &perm.request.session_id);

    let (title, description, bash_command_raw) = build_permission_display(
        &perm.request,
        bash_highlights.as_ref(),
        #[cfg(feature = "local-workspace")]
        matches!(
            agent.workspace_mode,
            crate::views::welcome::WelcomeWorkspaceMode::LocalWorkspace
        ),
        #[cfg(not(feature = "local-workspace"))]
        false,
    );

    let perm_id = agent.next_perm_req_id;
    agent.next_perm_req_id += 1;

    if agent.permission_queue.is_empty() && agent.permission_stashed_prompt.is_none() {
        agent.permission_stashed_prompt = Some(agent.prompt.stash());
        agent.prompt.set_text("");
    }

    if agent.permission_queue.is_empty()
        && agent.active_pane == AgentPane::Scrollback
        && agent.permission_stashed_pane.is_none()
    {
        agent.permission_stashed_pane = Some(AgentPane::Scrollback);
        agent.set_active_pane(AgentPane::Prompt, true);
    }

    let options = perm.request.options.clone();

    let active_idx = crate::appearance::permission_cursor::resolve_initial_cursor(&options);

    let is_shell = bash_highlights.is_some()
        || bash_command_raw.is_some()
        || perm.request.tool_call.fields.kind == Some(acp::ToolKind::Execute);
    let auto_review = auto_review_for_card(&perm.request, is_shell);

    agent.permission_queue.push_back(PermissionViewState {
        request: perm,
        id: perm_id,
        focus: PermissionFocus::Options,
        options,
        active_idx,
        bash_highlights,
        bash_selection_count,
        bash_deny_selection_count,
        bash_command_raw,
        mcp_scope,
        title,
        description,
        args_expanded: false,
        desc_scroll: 0,
        subagent_label,
        options_area_height: 0,
        options_scroll_offset: 0,
        auto_review,
        auto_review_explanation_open: false,
    });

    agent.last_active_at = Some(std::time::Instant::now());

    true
}

fn resolve_subagent_label(agent: &AgentView, session_id: &acp::SessionId) -> Option<String> {
    let sid = session_id.0.as_ref();
    if let Some(ref root_sid) = agent.session.session_id
        && root_sid.0.as_ref() == sid
    {
        return None;
    }
    if let Some(info) = agent.subagent_sessions.get(sid) {
        return Some(format!(
            "Subagent \"{}\" ({}):",
            info.description, info.subagent_type
        ));
    }
    Some("Child session (untracked):".to_string())
}

pub(super) fn build_permission_display(
    req: &acp::RequestPermissionRequest,
    bash_highlights: Option<&BashCommandHighlights>,
    session_local_workspace: bool,
) -> (String, Vec<String>, Option<String>) {
    let is_bash = bash_highlights.is_some();

    let bash_input = req.tool_call.fields.raw_input.as_ref().and_then(|v| {
        serde_json::from_value::<xai_grok_tools::implementations::BashToolInput>(v.clone()).ok()
    });

    let ask = hook_ask(req);
    let acp_title = req
        .tool_call
        .fields
        .title
        .as_deref()
        .map(|title| match &ask {
            Some(ask) => ask.strip_prompt_header(title),
            None => title,
        });

    let raw_command = bash_input.as_ref().map(|b| b.command.clone()).or_else(|| {
        acp_title
            .and_then(|t| t.strip_prefix("Execute `"))
            .and_then(|t| t.strip_suffix('`'))
            .map(|s| s.to_string())
    });

    let bash_description = bash_input.map(|b| b.description);

    let is_execute = is_bash
        || req.tool_call.fields.kind == Some(acp::ToolKind::Execute)
        || raw_command.is_some();

    let title = if is_execute {
        bash_description
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .unwrap_or_else(
                || match bash_highlights.and_then(|h| h.highlighted_words.first()) {
                    Some(bin) => format!("Allow `{bin}`?"),
                    None => "Allow Execute?".to_string(),
                },
            )
    } else if is_edit_permission(req) {
        let file_path = req
            .tool_call
            .fields
            .raw_input
            .as_ref()
            .and_then(|v| v.get("file_path"))
            .and_then(|v| v.as_str());
        if let Some(path) = file_path {
            format!("Allow Edit to {}?", path)
        } else if let Some(t) = acp_title {
            format!(
                "Allow {}?",
                xai_grok_workspace::permission::mcp_pretty_name_if_qualified(t)
            )
        } else {
            "Allow Edit?".to_string()
        }
    } else if let Some(t) = acp_title {
        format!(
            "Allow {}?",
            xai_grok_workspace::permission::mcp_pretty_name_if_qualified(t)
        )
    } else {
        match req.tool_call.fields.kind {
            Some(acp::ToolKind::Edit) => "Allow Edit?".to_string(),
            Some(acp::ToolKind::Execute) => "Allow Execute?".to_string(),
            Some(acp::ToolKind::Delete) => "Allow Delete?".to_string(),
            _ => "Allow?".to_string(),
        }
    };

    let title = qualify_permission_title_for_local_workspace(title, session_local_workspace);
    let description = permission_description_lines(req, ask.as_ref());
    let bash_cmd = if is_execute { raw_command } else { None };
    (title, description, bash_cmd)
}

fn qualify_permission_title_for_local_workspace(
    title: String,
    session_local_workspace: bool,
) -> String {
    if !session_local_workspace {
        return title;
    }
    if title.contains("on your machine") {
        return title;
    }
    if let Some(stripped) = title.strip_suffix('?') {
        return format!("{stripped} (on your machine)?");
    }
    format!("{title} (on your machine)")
}

fn permission_description_lines(
    req: &acp::RequestPermissionRequest,
    hook_ask: Option<&xai_grok_workspace::permission::HookAsk>,
) -> Vec<String> {
    let mut lines = mcp_args_lines(req);
    if is_edit_permission(req)
        && let Some(desc) = protected_edit_description(req)
    {
        lines.insert(0, desc);
    }
    if let Some(ask) = hook_ask {
        lines.insert(0, ask.ask_line());
    }
    lines
}

fn auto_review_for_card(
    req: &acp::RequestPermissionRequest,
    is_shell: bool,
) -> Option<xai_grok_workspace::permission::AutoReviewNotice> {
    if !is_shell {
        return None;
    }
    let value = req
        .meta
        .as_ref()?
        .get(xai_grok_workspace::permission::AUTO_REVIEW_META_KEY)?;
    serde_json::from_value(value.clone()).ok()
}

fn hook_ask(
    req: &acp::RequestPermissionRequest,
) -> Option<xai_grok_workspace::permission::HookAsk> {
    let value = req
        .meta
        .as_ref()?
        .get(xai_grok_workspace::permission::HOOK_ASK_META_KEY)?;
    serde_json::from_value(value.clone()).ok()
}

fn protected_edit_description(req: &acp::RequestPermissionRequest) -> Option<String> {
    let meta = req.meta.as_ref()?;
    let protected: xai_grok_workspace::permission::ProtectedEditPermission =
        serde_json::from_value(serde_json::Value::Object(meta.clone())).ok()?;
    protected.description.filter(|s| !s.is_empty())
}

pub(super) const MCP_ARGS_MAX_LINES: usize = 200;

pub(super) const MCP_ARGS_MAX_LINE_CHARS: usize = 2000;

pub(super) fn mcp_args_lines(req: &acp::RequestPermissionRequest) -> Vec<String> {
    let Some(raw) = req.tool_call.fields.raw_input.as_ref() else {
        return Vec::new();
    };
    let is_mcp = matches!(
        raw.get("variant").and_then(|v| v.as_str()),
        Some("UseTool") | Some("MCPTool")
    );
    if !is_mcp {
        return Vec::new();
    }
    // A tool that takes no arguments arrives as `{}`; there is nothing to show for it.
    let args = match raw.get("tool_input") {
        Some(serde_json::Value::Null) | None => return Vec::new(),
        Some(serde_json::Value::Object(map)) if map.is_empty() => return Vec::new(),
        Some(serde_json::Value::Array(items)) if items.is_empty() => return Vec::new(),
        Some(args) => args,
    };
    let pretty = match serde_json::to_string_pretty(args) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let mut lines: Vec<String> = pretty
        .lines()
        .map(|l| match l.char_indices().nth(MCP_ARGS_MAX_LINE_CHARS) {
            Some((byte_idx, _)) => match l.get(..byte_idx) {
                Some(prefix) => format!("{prefix}…"),
                None => l.to_owned(),
            },
            None => l.to_owned(),
        })
        .collect();
    if lines.len() > MCP_ARGS_MAX_LINES {
        let hidden = lines.len() - MCP_ARGS_MAX_LINES;
        lines.truncate(MCP_ARGS_MAX_LINES);
        lines.push(format!("… (+{hidden} more lines)"));
    }
    lines
}

fn is_edit_permission(req: &acp::RequestPermissionRequest) -> bool {
    req.options.iter().any(|o| {
        o.kind == acp::PermissionOptionKind::AllowAlways && o.name.to_lowercase().contains("edit")
    })
}

fn cancel_permission(perm: xai_acp_lib::AcpArgs<acp::RequestPermissionRequest>) {
    perm.response_tx
        .send(Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Cancelled,
        )))
        .ok();
}

pub(super) fn should_drop_late_auto_recap(
    auto: bool,
    is_replay: bool,
    agent: &crate::app::agent_view::AgentView,
) -> bool {
    auto && !is_replay && !cli_is_idle_for_recap(agent)
}

fn cli_is_idle_for_recap(agent: &crate::app::agent_view::AgentView) -> bool {
    if !agent.session.state.is_idle() || agent.has_wake_source() {
        return false;
    }
    if agent.session.in_flight_prompt.is_some() || agent.has_held_user_queue() {
        return false;
    }
    if scrollback_waiting_on_user_turn(&agent.scrollback) {
        return false;
    }
    true
}

fn scrollback_waiting_on_user_turn(scrollback: &crate::scrollback::state::ScrollbackState) -> bool {
    use crate::scrollback::block::RenderBlock;

    for idx in (0..scrollback.len()).rev() {
        let Some(entry) = scrollback.get(idx) else {
            continue;
        };
        if entry.block.is_user_prompt() {
            return true;
        }
        if matches!(
            &entry.block,
            RenderBlock::AgentMessage(_) | RenderBlock::Thinking(_) | RenderBlock::ToolCall(_)
        ) {
            return false;
        }
        if let RenderBlock::SessionEvent(b) = &entry.block
            && session_event_settles_turn(&b.event)
        {
            return false;
        }
    }
    false
}

fn session_event_settles_turn(event: &crate::scrollback::blocks::SessionEvent) -> bool {
    use crate::scrollback::blocks::SessionEvent;

    event.is_turn_terminal()
        || matches!(
            event,
            SessionEvent::ReAuthRequired
                | SessionEvent::ContextTooLarge
                | SessionEvent::DiskFull
                | SessionEvent::CompactionFailed { .. }
                | SessionEvent::RetryFailed { .. }
        )
}

pub(super) fn should_drop_duplicate_auto_recap(
    auto: bool,
    is_replay: bool,
    scrollback: &crate::scrollback::state::ScrollbackState,
) -> bool {
    auto && !is_replay && scrollback_has_recap_since_last_user(scrollback)
}

fn scrollback_has_recap_since_last_user(
    scrollback: &crate::scrollback::state::ScrollbackState,
) -> bool {
    use crate::scrollback::block::RenderBlock;
    use crate::scrollback::blocks::SessionEvent;

    let mut recap_since_user = false;
    for (_, entry) in scrollback.iter_entries() {
        if entry.block.is_user_prompt() {
            recap_since_user = false;
            continue;
        }
        if let RenderBlock::SessionEvent(b) = &entry.block
            && matches!(b.event, SessionEvent::Recap { .. })
        {
            recap_since_user = true;
        }
    }
    recap_since_user
}

pub(super) fn apply_recap_block(agent: &mut AgentView, auto: bool, recap_block: RenderBlock) {
    let fill_id = if auto {
        None
    } else {
        agent
            .pending_recap_entry
            .take()
            .filter(|&id| agent.scrollback.get_by_id(id).is_some())
    };
    match fill_id {
        Some(id) if agent.scrollback.is_committed(id) => {
            agent.scrollback.remove_entry(id);
            agent.scrollback.push_block(recap_block);
        }
        Some(id) => {
            if let Some(entry) = agent.scrollback.get_by_id_mut(id) {
                entry.block = recap_block;
            }
            agent.scrollback.finish_running(id);
        }
        None => {
            agent.scrollback.push_block(recap_block);
        }
    }
}

#[cfg(test)]
mod auto_review_card_tests {
    use super::*;
    use crate::actions::ActionRegistry;
    use crate::app::app_view::InputOutcome;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use xai_grok_workspace::permission::{
        AUTO_REVIEW_META_KEY, ClassifierFailure, auto_review_notice, reasons,
    };

    fn request_with_notice(
        raw_input: serde_json::Value,
        title: &str,
        notice: Option<&xai_grok_workspace::permission::AutoReviewNotice>,
    ) -> xai_acp_lib::AcpArgs<acp::RequestPermissionRequest> {
        let (response_tx, _rx) = tokio::sync::oneshot::channel();
        let mut meta = serde_json::Map::new();
        if let Some(notice) = notice {
            meta.insert(
                AUTO_REVIEW_META_KEY.to_owned(),
                serde_json::to_value(notice).unwrap(),
            );
        }
        let fields = acp::ToolCallUpdateFields::new()
            .raw_input(Some(raw_input))
            .title(Some(title.to_owned()));
        let request = acp::RequestPermissionRequest::new(
            acp::SessionId::new(std::sync::Arc::from("s1")),
            acp::ToolCallUpdate::new(acp::ToolCallId::new(std::sync::Arc::from("call-1")), fields),
            vec![acp::PermissionOption::new(
                acp::PermissionOptionId::new(std::sync::Arc::from("allow-once")),
                "Yes, proceed",
                acp::PermissionOptionKind::AllowOnce,
            )],
        )
        .meta((!meta.is_empty()).then_some(meta));
        xai_acp_lib::AcpArgs {
            request,
            response_tx,
        }
    }

    fn render_text(state: &PermissionViewState) -> String {
        let theme = crate::theme::Theme::current();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let _ = crate::views::permission_view::render_permission_view(
            &mut buf, area, state, "", None, None, &theme, true,
        );
        (0..area.height)
            .map(|row| {
                (0..area.width)
                    .map(|col| {
                        buf.cell((col, row))
                            .map(|cell| cell.symbol().to_string())
                            .unwrap_or_default()
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn ctrl_f(agent: &mut crate::app::agent_view::AgentView) -> InputOutcome {
        agent.pane_areas.prompt = Rect::new(0, 0, 80, 24);
        agent.handle_input(
            &Event::Key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)),
            &ActionRegistry::defaults(),
        )
    }

    #[test]
    fn shell_card_says_when_auto_review_blocked_and_ctrl_f_shows_the_explanation() {
        let notice = auto_review_notice(
            Some(reasons::AUTO_CLASSIFIER_DENY),
            Some("pushes to a remote"),
        )
        .expect("deny");
        let line = notice.card_line();
        let mut agent = crate::test_util::make_agent_view(Some("s1"), "/tmp");
        enqueue_permission(
            request_with_notice(
                serde_json::json!({
                    "command": "git push origin main",
                    "description": "Push the branch",
                }),
                "Execute `git push origin main`",
                Some(&notice),
            ),
            &mut agent,
        );
        let closed_h = {
            let card = agent.permission_queue.front().expect("shell card");
            assert_eq!(card.auto_review.as_ref(), Some(&notice));
            assert!(!card.auto_review_explanation_open);
            let closed = render_text(card);
            assert!(closed.contains(line), "blocked status missing:\n{closed}");
            assert!(
                !closed.contains("pushes to a remote"),
                "explanation stays closed until Ctrl+F:\n{closed}"
            );
            assert!(
                closed.contains("ctrl+f") && closed.contains("explanation"),
                "Ctrl+F hint missing:\n{closed}"
            );
            crate::views::permission_view::permission_view_height(card, 40, 80)
        };

        let outcome = ctrl_f(&mut agent);
        assert!(matches!(outcome, InputOutcome::Changed), "{outcome:?}");
        let card = agent.permission_queue.front().unwrap();
        assert!(card.auto_review_explanation_open);
        assert!(
            !card.args_expanded,
            "Ctrl+F opens the explanation, not the command"
        );
        let open = render_text(card);
        assert!(
            open.contains(line),
            "status stays while the explanation is open:\n{open}"
        );
        assert!(
            open.contains("pushes to a remote"),
            "explanation missing:\n{open}"
        );
        let open_h = crate::views::permission_view::permission_view_height(card, 40, 80);
        assert_eq!(
            closed_h, open_h,
            "opening the explanation must not resize the card"
        );
    }

    #[test]
    fn shell_card_says_when_auto_review_could_not_review() {
        let explanation = ClassifierFailure::Timeout.to_string();
        let notice = auto_review_notice(
            Some(reasons::AUTO_CLASSIFIER_TIMEOUT),
            Some(explanation.as_str()),
        )
        .expect("timeout");
        let line = notice.card_line();
        let mut agent = crate::test_util::make_agent_view(Some("s1"), "/tmp");
        enqueue_permission(
            request_with_notice(
                serde_json::json!({
                    "command": "git push origin main",
                    "description": "Push the branch",
                }),
                "Execute `git push origin main`",
                Some(&notice),
            ),
            &mut agent,
        );
        let closed = render_text(agent.permission_queue.front().unwrap());
        assert!(
            closed.contains(line),
            "couldn't-review status missing:\n{closed}"
        );
        assert!(
            !closed.contains(explanation.as_str()),
            "timeout text stays behind Ctrl+F:\n{closed}"
        );
        ctrl_f(&mut agent);
        let open = render_text(agent.permission_queue.front().unwrap());
        assert!(
            open.contains(explanation.as_str()),
            "timeout explanation missing:\n{open}"
        );
    }

    #[test]
    fn card_without_an_auto_review_notice_still_expands_a_long_command() {
        let command = (0..12)
            .map(|i| format!("echo line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut agent = crate::test_util::make_agent_view(Some("s1"), "/tmp");
        enqueue_permission(
            request_with_notice(
                serde_json::json!({
                    "command": command,
                    "description": "Run the script",
                }),
                "Execute script",
                None,
            ),
            &mut agent,
        );
        let card = agent.permission_queue.front().unwrap();
        assert!(card.auto_review.is_none());
        let collapsed = render_text(card);
        assert!(
            !collapsed.contains("Auto-review"),
            "a normal shell card stays quiet:\n{collapsed}"
        );
        assert!(
            !collapsed.contains("echo line4"),
            "the long command starts collapsed:\n{collapsed}"
        );
        let outcome = ctrl_f(&mut agent);
        assert!(matches!(outcome, InputOutcome::Changed), "{outcome:?}");
        let card = agent.permission_queue.front().unwrap();
        assert!(card.args_expanded);
        let expanded = render_text(card);
        assert!(
            expanded.contains("echo line11"),
            "Ctrl+F still expands a command that has no explanation:\n{expanded}"
        );
        assert!(!expanded.contains("Auto-review"), "{expanded}");
    }

    #[test]
    fn edit_card_does_not_show_an_auto_review_notice() {
        let notice = auto_review_notice(
            Some(reasons::AUTO_CLASSIFIER_DENY),
            Some("rewrites a protected file"),
        )
        .expect("deny");
        let mut agent = crate::test_util::make_agent_view(Some("s1"), "/tmp");
        enqueue_permission(
            request_with_notice(
                serde_json::json!({
                    "file_path": "/tmp/x.rs",
                    "old_string": "a",
                    "new_string": "b",
                }),
                "Edit /tmp/x.rs",
                Some(&notice),
            ),
            &mut agent,
        );
        let card = agent.permission_queue.front().unwrap();
        assert!(
            card.auto_review.is_none(),
            "edit cards do not name Auto-review"
        );
        let text = render_text(card);
        assert!(
            !text.contains(notice.card_line()),
            "edit card rendered the shell notice:\n{text}"
        );
    }
}

#[cfg(all(test, feature = "local-workspace"))]
mod tests {
    use super::*;

    #[test]
    fn permission_title_qualifies_for_local_workspace() {
        assert_eq!(
            qualify_permission_title_for_local_workspace("Allow Edit?".into(), false),
            "Allow Edit?"
        );
        assert_eq!(
            qualify_permission_title_for_local_workspace("Allow Edit?".into(), true),
            "Allow Edit (on your machine)?"
        );
    }
}
