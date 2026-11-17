//! Minimal-mode below-prompt list panels: `/resume` (session picker) and `/mcps` (MCP server status).
//! Both render as simple lists below the input bar instead of centered modal windows.
//!
//! ## Why this is a render-only change
//!
//! Input routing is unchanged.
//! `handle_modal_key` (`ActiveModal::SessionPicker`) and `handle_extensions_modal_key` (`extensions_modal`) own navigation and close-on-Esc.
//! The two panels couple to their input handlers differently:
//!
//! * **Session picker** rebuilds its entry map from data on every keypress (render-independent).
//!   The panel reuses the same builders ([`build_grouped_picker_entries`]), so the rendered order matches the handler's `selected`.
//! * **Extensions modal** reads render-stored state (`entry_data_indices`, `entry_group_keys`, `entry_non_selectable*`).
//!   The MCP renderer repopulates those exactly as the full modal does (via the shared [`build_mcp_servers_picker_rows`]).
//!   Keyboard nav and section fold then stay in sync without touching the input handler.
//!
//! Both reuse [`picker::render_picker_content`] for the rows, so row look and selection highlight match the full TUI.
//! Only the modal-window chrome (border, tabs, footer bar) is dropped.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use xai_grok_pager::app::agent_view::AgentView;
use xai_grok_pager::minimal_api;
use xai_grok_pager::theme::Theme;
use xai_grok_pager::views::extensions_modal::{ExtensionsTab, StatusFilter, TabDataState};
use xai_grok_pager::views::mcps_modal::{
    McpSectionId, McpServerInfo, section_description_lines, section_key,
};
use xai_grok_pager::views::modal::ActiveModal;
use xai_grok_pager::views::picker::{self, PickerEntry, PickerField, PickerHitAreas, PickerRow};

/// Rows of chrome around the scrolling list: title + subtitle/search + divider + footer.
const CHROME_ROWS: u16 = 4;

/// Which below-prompt list panel is active for the focused agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListPanel {
    /// `/resume` session picker (`ActiveModal::SessionPicker`).
    Resume,
    /// `/mcps` MCP server status (extensions modal on the McpServers tab).
    Mcps,
}

/// Detect an active below-prompt list panel, or `None`.
/// Only the session picker and the MCP-servers tab are hosted as simple lists; every other modal keeps its existing (centered) rendering.
/// Callers must check this *before* `overlay::app_modal_active`, since `SessionPicker` is also an `active_modal`.
pub(super) fn active(agent: &AgentView) -> Option<ListPanel> {
    if matches!(agent.active_modal, Some(ActiveModal::SessionPicker { .. })) {
        return Some(ListPanel::Resume);
    }
    if minimal_api::extensions_modal(agent)
        .is_some_and(|s| s.active_tab == ExtensionsTab::McpServers)
    {
        return Some(ListPanel::Mcps);
    }
    None
}

/// Target viewport height for the active list panel: chrome + the exact body height, clamped to `[CHROME_ROWS + 1, ceiling]`.
/// Sizing to the exact content height keeps the footer directly under the last row (no blank band).
/// When the body exceeds `ceiling` the list scrolls internally.
pub(super) fn panel_height(agent: &AgentView, kind: ListPanel, width: u16, ceiling: u16) -> u16 {
    let body = match kind {
        ListPanel::Resume => resume_body_rows(agent, width),
        ListPanel::Mcps => mcps_body_rows(agent),
    };
    CHROME_ROWS
        .saturating_add(body)
        .clamp(CHROME_ROWS + 1, ceiling.max(CHROME_ROWS + 1))
}

/// Render the active list panel into `area` (the whole live region).
/// `tick` advances the `/mcps` and `/resume` loading spinners the same way the full views do.
/// Returns the text cursor for the panel's search bar when search is focused, else `None`.
pub(super) fn render_with_tick(
    buf: &mut Buffer,
    area: Rect,
    agent: &mut AgentView,
    kind: ListPanel,
    theme: &Theme,
    tick: u64,
) -> Option<(u16, u16)> {
    if area.height < 2 || area.width < 8 {
        return None;
    }
    match kind {
        ListPanel::Resume => render_resume(buf, area, agent, theme, tick),
        ListPanel::Mcps => render_mcps(buf, area, agent, theme, tick),
    }
}

/// Split `area` into (title_row, second_row, divider_row, list_area, footer_row).
/// `second_row` hosts the subtitle (mcps) or the search bar (resume).
fn chrome_layout(area: Rect) -> (Rect, Rect, Rect, Rect, Rect) {
    let row = |dy: u16| Rect {
        x: area.x,
        y: area.y + dy,
        width: area.width,
        height: 1,
    };
    let title = row(0);
    let second = row(1);
    let divider = row(2);
    let footer = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        ..row(0)
    };
    let list = Rect {
        x: area.x,
        y: area.y + 3,
        width: area.width,
        height: area.height.saturating_sub(CHROME_ROWS),
    };
    (title, second, divider, list, footer)
}

fn render_title(buf: &mut Buffer, row: Rect, theme: &Theme, title: &str) {
    buf.set_style(row, Style::default().bg(Color::Reset));
    let style = Style::default()
        .fg(theme.accent_user)
        .bg(Color::Reset)
        .add_modifier(Modifier::BOLD);
    buf.set_span(row.x + 1, row.y, &Span::styled(title, style), row.width);
}

fn render_dim_line(buf: &mut Buffer, row: Rect, theme: &Theme, text: &str) {
    buf.set_style(row, Style::default().bg(Color::Reset));
    let style = theme.dim().bg(Color::Reset);
    buf.set_span(row.x + 1, row.y, &Span::styled(text, style), row.width);
}

/// `/resume` session picker: Enter picks a session.
const RESUME_FOOTER: &str = "\u{2191}/\u{2193} navigate \u{00b7} Enter confirm \u{00b7} Esc cancel";

/// Shown once `d` has armed a delete, and until `y` deletes or `n` cancels.
const RESUME_DELETE_FOOTER: &str = "y confirm delete \u{00b7} n cancel";

fn render_footer(buf: &mut Buffer, row: Rect, theme: &Theme, text: &str) {
    // The line is indented one column, so the hint budget is the rest of the row.
    let budget = usize::from(row.width.saturating_sub(1));
    let shown = fit_footer_hint(text, budget);
    render_dim_line(buf, row, theme, &shown);
}

const FOOTER_SEP: &str = " \u{00b7} ";

/// Keep whole hints, then whole words. A narrow row ends on a word or `…`, never inside a word.
fn fit_footer_hint(text: &str, budget: usize) -> String {
    if budget == 0 || text.is_empty() {
        return String::new();
    }
    if text.width() <= budget {
        return text.to_string();
    }
    let segments: Vec<&str> = text.split(FOOTER_SEP).collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut used = 0usize;
    for (i, seg) in segments.iter().enumerate() {
        let sep = if i == 0 { 0 } else { FOOTER_SEP.width() };
        if used.saturating_add(sep).saturating_add(seg.width()) <= budget {
            used = used.saturating_add(sep).saturating_add(seg.width());
            kept.push(*seg);
            continue;
        }
        let remain = budget.saturating_sub(used.saturating_add(sep));
        let partial = fit_footer_words(seg, remain);
        if partial.is_empty() {
            break;
        }
        let mut out = kept.join(FOOTER_SEP);
        if !out.is_empty() {
            out.push_str(FOOTER_SEP);
        }
        out.push_str(&partial);
        return out;
    }
    let mut out = kept.join(FOOTER_SEP);
    if kept.len() != segments.len() {
        let mark = if out.is_empty() { "…" } else { " …" };
        if out.width().saturating_add(mark.width()) <= budget {
            out.push_str(mark);
        }
    }
    out
}

/// Whole words of one hint, then `…` when the rest does not fit.
fn fit_footer_words(seg: &str, budget: usize) -> String {
    if budget == 0 {
        return String::new();
    }
    if seg.width() <= budget {
        return seg.to_string();
    }
    if budget == 1 {
        return "…".to_string();
    }
    let mut kept = String::new();
    let mut kept_w = 0usize;
    for word in seg.split_whitespace() {
        let word_w = word.width();
        let gap = usize::from(!kept.is_empty());
        if kept_w.saturating_add(gap).saturating_add(word_w).saturating_add(1) > budget {
            break;
        }
        if gap == 1 {
            kept.push(' ');
        }
        kept.push_str(word);
        kept_w = kept_w.saturating_add(gap).saturating_add(word_w);
    }
    if kept.is_empty() {
        return "…".to_string();
    }
    kept.push('…');
    kept
}

fn render_divider(buf: &mut Buffer, row: Rect, theme: &Theme) {
    picker::render_divider(buf, row.x, row.y, row.width, theme, None);
}

/// Exact body height (display rows) for the session-picker list.
fn resume_body_rows(agent: &AgentView, width: u16) -> u16 {
    let Some(ActiveModal::SessionPicker {
        entries,
        state,
        source_filter,
        entries_query,
        grouped,
        ..
    }) = &agent.active_modal
    else {
        return 0;
    };
    let entries_data = entries.as_deref().unwrap_or(&[]);
    let content_width = width.saturating_sub(2);
    // Same query the key map uses, so a server search that matched message text stays in the list.
    let filter_query =
        minimal_api::effective_filter_query(state.query(), entries_query.as_deref());
    let filtered =
        minimal_api::filter_session_entries(entries.as_deref(), filter_query, *source_filter);
    let built =
        minimal_api::build_session_entry_data(entries_data, &filtered, state, content_width);
    let fields_vecs: Vec<Vec<PickerField>> = built
        .iter()
        .map(|b| {
            b.field_data
                .iter()
                .map(|(l, v)| PickerField { label: l, value: v })
                .collect()
        })
        .collect();
    let current_repo = minimal_api::repo_name_from_cwd(&agent.session.cwd.to_string_lossy());
    let (picker_entries, _) = minimal_api::build_visible_picker_entries(
        *grouped,
        entries_data,
        &filtered,
        &built,
        &fields_vecs,
        state,
        Some(current_repo.as_str()),
    );
    // Reserve a row for the pinned hidden-external hint when shown.
    let hint_row = u16::from(
        !agent.app_chat_mode
            && minimal_api::hidden_external_hint(entries.as_deref(), *source_filter).is_some(),
    );
    measure_entries(&picker_entries, width).saturating_add(hint_row)
}

fn render_resume(
    buf: &mut Buffer,
    area: Rect,
    agent: &mut AgentView,
    theme: &Theme,
    tick: u64,
) -> Option<(u16, u16)> {
    let cwd = agent.session.cwd.to_string_lossy().to_string();
    let chat_mode = agent.app_chat_mode;
    let Some(ActiveModal::SessionPicker {
        entries,
        state,
        source_filter,
        pending_delete,
        loading,
        lanes,
        entries_query,
        grouped,
        ..
    }) = &mut agent.active_modal
    else {
        return None;
    };
    let delete_armed = pending_delete.is_some();
    let (title_row, search_row, divider_row, mut list_area, footer_row) = chrome_layout(area);

    let entries_data = entries.as_deref().unwrap_or(&[]);
    let content_width = area.width.saturating_sub(2);
    // Keys call `effective_filter_query`. A matching server stamp skips the local title filter
    // so message-text hits whose titles do not contain the query stay on screen.
    let filter_query =
        minimal_api::effective_filter_query(state.query(), entries_query.as_deref());
    let filtered =
        minimal_api::filter_session_entries(entries.as_deref(), filter_query, *source_filter);
    let built =
        minimal_api::build_session_entry_data(entries_data, &filtered, state, content_width);
    let fields_vecs: Vec<Vec<PickerField>> = built
        .iter()
        .map(|b| {
            b.field_data
                .iter()
                .map(|(l, v)| PickerField { label: l, value: v })
                .collect()
        })
        .collect();
    let current_repo = minimal_api::repo_name_from_cwd(&cwd);
    let (picker_entries, non_sel) = minimal_api::build_visible_picker_entries(
        *grouped,
        entries_data,
        &filtered,
        &built,
        &fields_vecs,
        state,
        Some(current_repo.as_str()),
    );
    state.empty_label =
        minimal_api::session_picker_empty_label(entries.as_deref(), entries_query.as_deref());
    let show_spinner = minimal_api::loading_spinner_active(
        entries.as_deref(),
        *source_filter,
        *loading,
        lanes,
    );
    let hidden_hint = (!chat_mode)
        .then(|| minimal_api::hidden_external_hint(entries.as_deref(), *source_filter))
        .flatten();

    render_title(buf, title_row, theme, "Resume session");
    // The source-filter chip uses the full resume row: it stays when the width allows and stays off the query when it does not.
    let search_area = Rect::new(
        search_row.x + 1,
        search_row.y,
        search_row.width.saturating_sub(1),
        1,
    );
    let filter = (!chat_mode).then_some((source_filter.label(), "f", source_filter.is_active()));
    let filter_area =
        minimal_api::render_resume_search_row(buf, search_area, theme, state, filter);
    state.filter_area = filter_area;
    render_divider(buf, divider_row, theme);

    // Pinned above the list so it stays visible regardless of list scroll.
    if let Some(hint) = hidden_hint.as_deref() {
        render_dim_line(
            buf,
            Rect {
                height: 1,
                ..list_area
            },
            theme,
            hint,
        );
        list_area.y += 1;
        list_area.height = list_area.height.saturating_sub(1);
    }

    let nsc = vec![false; picker_entries.len()];
    let hit = picker::render_picker_content_with_tick(
        buf,
        list_area,
        theme,
        state,
        &picker_entries,
        &non_sel,
        &nsc,
        None,
        show_spinner,
        tick,
    );
    state.hit_areas = Some(PickerHitAreas {
        close_button: Rect::default(),
        search_bar: search_row,
        item_rects: hit.item_rects,
        entry_indices: hit.entry_indices,
        tab_rects: vec![],
        filter_rect: None,
    });

    render_footer(
        buf,
        footer_row,
        theme,
        if delete_armed {
            RESUME_DELETE_FOOTER
        } else {
            RESUME_FOOTER
        },
    );
    None
}

/// Collapsed sections the next paint will use. Plugin sections start collapsed on the
/// first load, matching the full modal, even before that render has written the set back.
fn collapsed_mcp_sections(
    collapsed: &std::collections::HashSet<String>,
    initialized: bool,
    servers: &[McpServerInfo],
) -> std::collections::HashSet<String> {
    let mut collapsed = collapsed.clone();
    let mut initialized = initialized;
    minimal_api::init_mcps_section_collapse_on_first_load(&mut collapsed, &mut initialized, servers);
    collapsed
}

fn seed_mcp_plugin_sections(agent: &mut AgentView) {
    let Some(s) = minimal_api::extensions_modal_mut(agent) else {
        return;
    };
    if s.mcps_section_collapse_initialized {
        return;
    }
    let TabDataState::Loaded(servers) = &s.mcps_data else {
        return;
    };
    let servers = servers.clone();
    minimal_api::init_mcps_section_collapse_on_first_load(
        &mut s.mcps_collapsed_sections,
        &mut s.mcps_section_collapse_initialized,
        &servers,
    );
}

/// Exact body height (display rows) for the MCP list.
/// A loading spinner or an error row is one line. A loaded list counts section copy,
/// each server's tool summary, and an opened tool's description.
fn mcps_body_rows(agent: &AgentView) -> u16 {
    let Some(s) = minimal_api::extensions_modal(agent) else {
        return 0;
    };
    if s.input.is_some() {
        // Two add-form fields: label + bordered row each, plus the gap between them.
        return 9;
    }
    if let Some(setup) = s.mcp_setup.as_ref() {
        return (setup.field.options.len() as u16).saturating_add(4).max(6);
    }
    let servers = match &s.mcps_data {
        TabDataState::Loaded(v) => v.as_slice(),
        _ => return 1,
    };
    let collapsed = collapsed_mcp_sections(
        &s.mcps_collapsed_sections,
        s.mcps_section_collapse_initialized,
        servers,
    );
    let searching = !s.picker_state.query().is_empty();
    let (labels, keys, indices) = minimal_api::build_mcp_picker_rows(
        servers,
        s.picker_state.query(),
        s.mcps_filter,
        &collapsed,
        &s.mcps_tools_expanded,
    );
    let managed = section_key(&McpSectionId::Managed);
    let mut rows = labels.len() as u16;
    for (i, key) in keys.iter().enumerate() {
        match key.as_deref() {
            Some(k) if k.starts_with("mcp-tools:") => rows += 1,
            Some(k) if k == managed => {
                let hidden =
                    minimal_api::mcp_section_children_hidden(&collapsed, k, searching);
                if !hidden {
                    rows += section_description_lines(
                        &McpSectionId::Managed,
                        s.session_team_id.as_deref(),
                    )
                    .len() as u16;
                }
            }
            None if s.picker_state.expanded.contains(&i) => {
                let Some(si) = indices.get(i).copied().flatten() else {
                    continue;
                };
                let label = labels.get(i).map(String::as_str).unwrap_or("");
                let described = servers.get(si).is_some_and(|srv| {
                    srv.tools.iter().any(|tool| {
                        tool.display_name.as_deref().unwrap_or(tool.name.as_str()) == label
                            && tool
                                .description
                                .as_deref()
                                .is_some_and(|desc| !desc.is_empty())
                    })
                });
                if described {
                    rows += 1;
                }
            }
            _ => {}
        }
    }
    rows
}

fn render_mcps(
    buf: &mut Buffer,
    area: Rect,
    agent: &mut AgentView,
    theme: &Theme,
    tick: u64,
) -> Option<(u16, u16)> {
    seed_mcp_plugin_sections(agent);
    let (title_row, subtitle_row, divider_row, list_area, footer_row) = chrome_layout(area);
    render_title(buf, title_row, theme, "Manage MCP servers");

    // Phase 1 (immutable): build the row mapping and owned per-row render data
    let labels: Vec<String>;
    let group_keys: Vec<Option<String>>;
    let data_indices: Vec<Option<usize>>;
    let badges: Vec<String>;
    let badge_colors: Vec<Option<Color>>;
    let right_labels: Vec<String>;
    let indents: Vec<u8>;
    let collapsibles: Vec<bool>;
    let expandeds: Vec<bool>;
    let summaries: Vec<Vec<String>>;
    let descriptions: Vec<Vec<String>>;
    let dimmeds: Vec<bool>;
    let underlines: Vec<bool>;
    let loading;
    {
        let s = minimal_api::extensions_modal(agent)?;
        let searching = !s.picker_state.query().is_empty();
        loading = matches!(s.mcps_data, TabDataState::Loading);
        match &s.mcps_data {
            TabDataState::Loaded(servers) => {
                let (row_labels, row_group_keys, row_data_indices) =
                    minimal_api::build_mcp_picker_rows(
                        servers,
                        s.picker_state.query(),
                        s.mcps_filter,
                        &s.mcps_collapsed_sections,
                        &s.mcps_tools_expanded,
                    );
                let n = row_labels.len();
                let mut b = vec![String::new(); n];
                let mut bc: Vec<Option<Color>> = vec![None; n];
                let mut rl = vec![String::new(); n];
                let mut ind = vec![0u8; n];
                let mut col = vec![false; n];
                let mut exp = vec![false; n];
                let mut summary_lines = vec![Vec::new(); n];
                let mut desc_lines = vec![Vec::new(); n];
                let mut dim = vec![false; n];
                let mut underline = vec![false; n];
                let managed = section_key(&McpSectionId::Managed);
                for i in 0..n {
                    let gk = row_group_keys.get(i).and_then(|k| k.as_deref());
                    if let Some(k) = gk.filter(|k| k.starts_with("mcp-section:")) {
                        if let Some(c) = col.get_mut(i) {
                            *c = true;
                        }
                        if let Some(e) = exp.get_mut(i) {
                            *e = !minimal_api::mcp_section_children_hidden(
                                &s.mcps_collapsed_sections,
                                k,
                                searching,
                            );
                        }
                        if k == managed {
                            if let Some(lines) = desc_lines.get_mut(i) {
                                *lines = section_description_lines(
                                    &McpSectionId::Managed,
                                    s.session_team_id.as_deref(),
                                );
                            }
                            if let Some(flag) = underline.get_mut(i) {
                                *flag = true;
                            }
                        }
                    } else if gk.is_some_and(|k| k.starts_with("mcp-tools:")) {
                        if let Some(d) = ind.get_mut(i) {
                            *d = 1;
                        }
                        if let Some(c) = col.get_mut(i) {
                            *c = true;
                        }
                        if let Some(si) = row_data_indices.get(i).copied().flatten() {
                            if let Some(e) = exp.get_mut(i) {
                                *e = s.mcps_tools_expanded.contains(&si);
                            }
                            if let Some(srv) = servers.get(si) {
                                let (badge, color) = minimal_api::mcp_row_badge(srv, theme);
                                if let Some(slot) = b.get_mut(i) {
                                    *slot = badge;
                                }
                                if let Some(slot) = bc.get_mut(i) {
                                    *slot = color;
                                }
                                if let Some(slot) = dim.get_mut(i) {
                                    *slot = !srv.enabled;
                                }
                                if let Some(right) = rl.get_mut(i) {
                                    *right = minimal_api::mcp_server_source_label(srv);
                                }
                                let summary = minimal_api::mcp_server_tool_summary(srv);
                                if let Some(lines) = summary_lines.get_mut(i) {
                                    lines.push(summary.clone());
                                }
                                if let Some(lines) = desc_lines.get_mut(i) {
                                    lines.push(summary);
                                }
                            }
                        }
                    } else if let Some(d) = ind.get_mut(i) {
                        *d = 2; // tool child
                        if let Some(si) = row_data_indices.get(i).copied().flatten() {
                            if let Some(srv) = servers.get(si) {
                                let label = row_labels.get(i).map(String::as_str).unwrap_or("");
                                if let Some(tool) = srv.tools.iter().find(|tool| {
                                    tool.display_name.as_deref().unwrap_or(tool.name.as_str())
                                        == label
                                }) {
                                    if !tool.enabled {
                                        if let Some(slot) = b.get_mut(i) {
                                            *slot = "[disabled]".to_string();
                                        }
                                        if let Some(slot) = bc.get_mut(i) {
                                            *slot = Some(theme.accent_error);
                                        }
                                        if let Some(slot) = dim.get_mut(i) {
                                            *slot = true;
                                        }
                                    }
                                    if let Some(desc) = tool
                                        .description
                                        .as_deref()
                                        .filter(|desc| !desc.is_empty())
                                    {
                                        if let Some(lines) = desc_lines.get_mut(i) {
                                            lines.push(desc.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                labels = row_labels;
                group_keys = row_group_keys;
                data_indices = row_data_indices;
                badges = b;
                badge_colors = bc;
                right_labels = rl;
                indents = ind;
                collapsibles = col;
                expandeds = exp;
                summaries = summary_lines;
                descriptions = desc_lines;
                dimmeds = dim;
                underlines = underline;
            }
            TabDataState::Loading => {
                labels = vec![];
                group_keys = vec![];
                data_indices = vec![];
                badges = vec![];
                badge_colors = vec![];
                right_labels = vec![];
                indents = vec![];
                collapsibles = vec![];
                expandeds = vec![];
                summaries = vec![];
                descriptions = vec![];
                dimmeds = vec![];
                underlines = vec![];
            }
            TabDataState::Error(msg) => {
                labels = vec![format!("Error: {msg}")];
                group_keys = vec![None];
                data_indices = vec![None];
                badges = vec![String::new()];
                badge_colors = vec![None];
                right_labels = vec![String::new()];
                indents = vec![0];
                collapsibles = vec![false];
                expandeds = vec![false];
                summaries = vec![Vec::new()];
                descriptions = vec![Vec::new()];
                dimmeds = vec![false];
                underlines = vec![false];
            }
        }
    }
    let n = labels.len();

    // Phase 2 (mutable): mirror the row mapping onto state for the input handler.
    {
        let s = minimal_api::extensions_modal_mut(agent)?;
        s.entry_data_indices = data_indices;
        s.entry_group_keys = group_keys;
        s.entry_labels_cache = labels.clone();
        s.entry_non_selectable = vec![false; n];
        s.entry_non_selectable_clickable = vec![false; n];
        if n == 0 {
            s.picker_state.selected = 0;
        } else if s.picker_state.selected >= n {
            s.picker_state.selected = n - 1;
        }
    }

    let form_open = minimal_api::extensions_modal(agent)
        .is_some_and(|s| s.input.is_some() || s.mcp_setup.is_some());
    let mut search_rect = Rect::default();
    let mut filter_rect = None;
    if !form_open {
        if let Some(s) = minimal_api::extensions_modal(agent) {
            let filter = s.mcps_filter;
            let row = Rect::new(
                subtitle_row.x + 1,
                subtitle_row.y,
                subtitle_row.width.saturating_sub(1),
                1,
            );
            let filter_w = picker::filter_indicator_width(filter.label(), "f");
            minimal_api::render_picker_search_bar_with_trailing(
                buf,
                row,
                theme,
                &s.picker_state,
                true,
                None,
                filter_w,
            );
            let search_w = picker::search_bar_layout(row.width, filter_w).render_width();
            search_rect = Rect::new(row.x, row.y, search_w, 1);
            filter_rect = Some(picker::render_filter_indicator(
                buf,
                row.x,
                row.y,
                row.width,
                theme,
                filter.label(),
                "f",
                filter != StatusFilter::All,
                s.picker_state.filter_hovered,
            ));
        }
        render_divider(buf, divider_row, theme);
    } else {
        let form_area = Rect {
            x: area.x,
            y: subtitle_row.y,
            width: area.width,
            height: footer_row.y.saturating_sub(subtitle_row.y),
        };
        let footer = {
            let s = minimal_api::extensions_modal(agent)?;
            if let Some(setup) = s.mcp_setup.as_ref() {
                minimal_api::render_mcp_setup_form(buf, form_area, setup, theme);
            } else if let Some(input) = s.input.as_ref() {
                minimal_api::render_extensions_input_form(buf, form_area, input, theme);
            }
            minimal_api::mcp_panel_footer_labels(s).join(" \u{00b7} ")
        };
        render_footer(buf, footer_row, theme, &footer);
        if let Some(s) = minimal_api::extensions_modal_mut(agent) {
            s.picker_state.hit_areas = Some(PickerHitAreas {
                close_button: Rect::default(),
                search_bar: Rect::default(),
                item_rects: vec![],
                entry_indices: vec![],
                tab_rects: vec![],
                filter_rect: None,
            });
        }
        return None;
    }

    // Phase 3 (mutable picker_state): build PickerEntry from owned data and render
    let s = minimal_api::extensions_modal_mut(agent)?;
    let selected = s.picker_state.selected;
    let search_active = s.picker_state.search_active;
    let opened_rows = s.picker_state.expanded.clone();
    let empty_fields: [PickerField; 0] = [];
    let summary_refs: Vec<Vec<&str>> = summaries
        .iter()
        .map(|lines| lines.iter().map(String::as_str).collect())
        .collect();
    let desc_refs: Vec<Vec<&str>> = descriptions
        .iter()
        .map(|lines| lines.iter().map(String::as_str).collect())
        .collect();
    let entries: Vec<PickerEntry> = (0..n)
        .map(|i| {
            let collapsible = collapsibles.get(i).copied().unwrap_or(false);
            let expanded = if collapsible {
                expandeds.get(i).copied().unwrap_or(false)
            } else {
                opened_rows.contains(&i)
            };
            PickerEntry::Row(PickerRow {
                label: labels.get(i).map(String::as_str).unwrap_or(""),
                right_label: right_labels.get(i).map(String::as_str).unwrap_or(""),
                selected: !search_active && i == selected,
                expanded,
                fields: &empty_fields,
                description_lines: desc_refs.get(i).map(Vec::as_slice).unwrap_or(&[]),
                summary_lines: summary_refs.get(i).map(Vec::as_slice).unwrap_or(&[]),
                dimmed: dimmeds.get(i).copied().unwrap_or(false),
                indent: indents.get(i).copied().unwrap_or(0),
                badge: badges.get(i).map(String::as_str).unwrap_or(""),
                badge_color: badge_colors.get(i).copied().flatten(),
                collapsible,
                underline_last_desc: underlines.get(i).copied().unwrap_or(false),
            })
        })
        .collect();
    let non_sel = vec![false; n];
    let hit = picker::render_picker_content_with_tick(
        buf,
        list_area,
        theme,
        &mut s.picker_state,
        &entries,
        &non_sel,
        &non_sel,
        None,
        loading,
        tick,
    );
    let footer = minimal_api::mcp_panel_footer_labels(s).join(" \u{00b7} ");
    s.picker_state.hit_areas = Some(PickerHitAreas {
        close_button: Rect::default(),
        search_bar: search_rect,
        item_rects: hit.item_rects,
        entry_indices: hit.entry_indices,
        tab_rects: vec![],
        filter_rect,
    });

    render_footer(buf, footer_row, theme, &footer);
    None
}

/// Sum the display height of grouped picker entries at `width`.
/// A header is one row, plus the blank spacer `render_picker_content` draws before non-first headers.
/// Row height matches [`picker::compute_row_height`], including a CWD value that wraps onto extra lines.
fn measure_entries(entries: &[PickerEntry<'_>], width: u16) -> u16 {
    entries
        .iter()
        .enumerate()
        .map(|(idx, e)| match e {
            PickerEntry::Header { .. } => {
                if idx == 0 {
                    1u16
                } else {
                    2u16
                }
            }
            PickerEntry::Row(row) => picker::compute_row_height(row, width) as u16,
        })
        .fold(0u16, |acc, h| acc.saturating_add(h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;
    use xai_grok_pager::views::extensions_modal::ExtensionsModalState;
    use xai_grok_pager::views::mcps_modal::{
        McpServerDisplayStatus, McpServerInfo, McpSetupConfig, McpSetupField, McpSetupOption,
        McpToolDetail, McpWireSource,
    };

    fn agent() -> AgentView {
        minimal_api::test_agent_view(Some("s1"), std::path::PathBuf::from("/tmp/repo"))
    }

    fn render(
        buf: &mut Buffer,
        area: Rect,
        agent: &mut AgentView,
        kind: ListPanel,
        theme: &Theme,
    ) {
        render_with_tick(buf, area, agent, kind, theme, 0);
    }

    #[test]
    fn minimal_footer_hints_do_not_truncate_mid_word() {
        let source = "Tab tabs \u{00b7} r refresh \u{00b7} a add \u{00b7} Space enable/disable \u{00b7} Esc close";
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let theme = Theme::current();
        render_footer(&mut buf, area, &theme, source);
        let painted: String = (0..area.width)
            .filter_map(|x| buf.cell((x, 0)).map(|cell| cell.symbol().to_string()))
            .collect();
        let shown = painted.trim();
        assert!(
            shown.contains("Tab"),
            "the footer still shows a hint: {painted:?}"
        );
        for word in shown.split_whitespace() {
            let word = word.trim_end_matches('\u{2026}').trim_matches('\u{00b7}');
            if word.is_empty() {
                continue;
            }
            assert!(
                source.split_whitespace().any(|src| src == word),
                "a footer hint was cut mid-word: {word:?} in {painted:?}"
            );
        }
    }

    fn tool_detail(name: &str, enabled: bool) -> McpToolDetail {
        McpToolDetail {
            name: name.to_string(),
            display_name: None,
            description: None,
            enabled,
        }
    }

    fn mcp_server(name: &str, status: McpServerDisplayStatus, tools: usize) -> McpServerInfo {
        McpServerInfo {
            name: name.to_string(),
            display_name: None,
            status,
            tool_count: tools,
            auth_required: false,
            setup_required: false,
            setup: None,
            setup_values: std::collections::HashMap::new(),
            tools: Vec::new(),
            enabled: true,
            blocked_reason: None,
            source: "local".to_string(),
            wire_source: McpWireSource::Local,
            plugin_name: None,
            is_managed_gateway: false,
        }
    }

    fn with_mcps(servers: Vec<McpServerInfo>) -> AgentView {
        let mut a = agent();
        minimal_api::set_extensions_modal(
            &mut a,
            Some(ExtensionsModalState {
                active_tab: ExtensionsTab::McpServers,
                mcps_data: TabDataState::Loaded(servers),
                ..Default::default()
            }),
        );
        a
    }

    fn session_entry(id: &str) -> xai_grok_pager::app::app_view::SessionPickerEntry {
        xai_grok_pager::app::app_view::SessionPickerEntry {
            id: id.into(),
            summary: id.into(),
            updated_at: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
            cwd: "/tmp/repo".into(),
            hostname: None,
            source: String::new(),
            model_id: None,
            num_messages: 0,
            last_active_at: None,
            branch: None,
            repo_name: "repo".into(),
            worktree_label: None,
            last_turn_summary: None,
            last_recap: None,
            session_kind: None,
            card_detail: None,
        }
    }

    fn with_resume(entries: Vec<xai_grok_pager::app::app_view::SessionPickerEntry>) -> AgentView {
        let mut a = agent();
        a.active_modal = Some(ActiveModal::SessionPicker {
            state: picker::PickerState::default(),
            entries: Some(entries),
            loading: false,
            lanes: Default::default(),
            previous_palette: None,
            window: xai_grok_pager::views::modal_window::ModalWindowState::new(),
            content_results: None,
            content_loading: false,
            deep_search_seq: 0,
            generation: 0,
            detail_seq: 0,
            source_filter: xai_grok_pager::views::session_picker::SourceFilter::default(),
            pending_delete: None,
            entries_query: None,
            grouped: true,
            next_cursor: None,
            paging: false,
        });
        a
    }

    #[test]
    fn active_detects_resume_mcps_and_none() {
        assert_eq!(active(&agent()), None);
        assert_eq!(
            active(&with_mcps(vec![mcp_server(
                "alpha",
                McpServerDisplayStatus::Ready,
                3
            )])),
            Some(ListPanel::Mcps)
        );
        assert_eq!(
            active(&with_resume(vec![session_entry("hello")])),
            Some(ListPanel::Resume)
        );
    }

    #[test]
    fn mcps_panel_renders_list_and_mirrors_handler_state() {
        let mut a = with_mcps(vec![
            mcp_server("alpha", McpServerDisplayStatus::Ready, 3),
            mcp_server("bravo", McpServerDisplayStatus::Unavailable, 0),
        ]);
        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render(&mut buf, area, &mut a, ListPanel::Mcps, &theme);

        let text = crate::buffer_text(&buf);
        assert!(text.contains("Manage MCP servers"), "title:\n{text}");
        assert!(text.contains("/ to search"), "search row:\n{text}");
        assert!(text.contains("alpha"), "server row:\n{text}");
        assert!(text.contains("bravo"), "server row:\n{text}");
        assert!(text.contains("Space enable/disable"), "footer:\n{text}");
        assert!(text.contains("r refresh"), "footer:\n{text}");
        assert!(text.contains("a add"), "footer:\n{text}");
        assert!(
            !text.contains("Enter confirm"),
            "MCP footer must not reuse resume confirm copy:\n{text}"
        );

        // The input handler reads these render-stored fields
        // The panel must mirror them (section header + 2 servers = 3 rows) so keyboard nav and fold stay correct without touching the handler
        let s = minimal_api::extensions_modal(&a).unwrap();
        assert_eq!(s.entry_data_indices.len(), 3, "section + 2 servers");
        assert_eq!(
            s.entry_data_indices.iter().filter(|d| d.is_some()).count(),
            2,
            "two selectable server rows map to catalog indices"
        );
        assert_eq!(s.entry_non_selectable.len(), 3);
    }

    #[test]
    fn mcps_panel_shows_source_and_tool_summary() {
        let mut listed = mcp_server("alpha", McpServerDisplayStatus::Ready, 4);
        listed.source = "plugin: acme".into();
        listed.tools = vec![
            tool_detail("read", true),
            tool_detail("write", true),
            tool_detail("hidden", false),
            tool_detail("off", false),
        ];
        let mut disabled = mcp_server("bravo", McpServerDisplayStatus::Unavailable, 0);
        disabled.enabled = false;
        disabled.source = "local".into();
        let mut a = with_mcps(vec![listed, disabled]);
        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render(&mut buf, area, &mut a, ListPanel::Mcps, &theme);
        let text = crate::buffer_text(&buf);
        assert!(
            text.contains("(plugin: acme)"),
            "server row keeps its source:\n{text}"
        );
        assert!(
            text.contains("4 tools (2 enabled)"),
            "tool summary names how many are enabled:\n{text}"
        );
        assert!(
            text.contains("(local)"),
            "a disabled server still names its source:\n{text}"
        );
        assert!(
            text.contains("no tools — server is disabled"),
            "an empty disabled server says why:\n{text}"
        );
    }

    #[test]
    fn mcps_panel_badges_policy_block_over_disabled() {
        let mut denied = mcp_server("denied-srv", McpServerDisplayStatus::BlockedByPolicy, 0);
        denied.enabled = false;
        denied.blocked_reason = Some("matches deniedMcpServers".into());
        let mut manual = mcp_server("manual-srv", McpServerDisplayStatus::Ready, 2);
        manual.enabled = false;
        let mut a = with_mcps(vec![denied, manual]);
        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render(&mut buf, area, &mut a, ListPanel::Mcps, &theme);

        let text = crate::buffer_text(&buf);
        let row = |name: &str| {
            text.lines()
                .find(|l| l.contains(name))
                .map(str::to_string)
                .unwrap_or_else(|| panic!("no row for {name}:\n{text}"))
        };
        let denied_row = row("denied-srv");
        assert!(
            denied_row.contains("blocked by policy"),
            "policy verdict must win:\n{denied_row}"
        );
        assert!(
            !denied_row.contains("disabled"),
            "policy-blocked row must not read as a personal disable:\n{denied_row}"
        );
        assert!(
            row("manual-srv").contains("disabled"),
            "personal disable keeps its badge:\n{text}"
        );
    }

    #[test]
    fn resume_delete_confirm_shows_y_and_n_before_the_session_goes() {
        let mut agent = with_resume(vec![session_entry("keep-me")]);
        let Some(ActiveModal::SessionPicker { pending_delete, .. }) = &mut agent.active_modal
        else {
            panic!("expected session picker");
        };
        *pending_delete = Some(xai_grok_pager::views::session_picker::PendingDelete {
            source: "local".into(),
            session_id: "keep-me".into(),
            cwd: "/tmp/repo".into(),
        });

        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render(&mut buf, area, &mut agent, ListPanel::Resume, &theme);

        let text = crate::buffer_text(&buf);
        assert!(
            text.contains("y confirm delete \u{00b7} n cancel"),
            "armed delete must name y and n before the session is removed:\n{text}"
        );
        assert!(
            text.contains("keep-me"),
            "the session stays listed until y confirms:\n{text}"
        );
        assert!(
            !text.contains("Enter confirm"),
            "the confirm line replaces the ordinary resume footer:\n{text}"
        );
    }

    #[test]
    fn resume_panel_renders_title_rows_and_footer() {
        let mut a = with_resume(vec![session_entry("first task"), session_entry("second")]);
        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render(&mut buf, area, &mut a, ListPanel::Resume, &theme);

        let text = crate::buffer_text(&buf);
        assert!(text.contains("Resume session"), "title:\n{text}");
        assert!(text.contains("first task"), "session row:\n{text}");
        assert!(text.contains("Enter confirm"), "resume footer:\n{text}");
        assert!(
            !text.contains("r refresh"),
            "resume footer must stay session-picker copy:\n{text}"
        );
        assert!(
            !text.contains("Loading\u{2026}"),
            "a settled list must not show the loading spinner:\n{text}"
        );
    }

    /// Minimal `/resume` uses the full picker's spinner and empty copy (`v88-x38`).
    #[test]
    fn minimal_resume_loading_and_empty_match_the_full_picker() {
        let theme = Theme::groknight();
        let frames = xai_grok_pager::glyphs::dot_spinner_frames();
        let frame0 = frames.first().copied().unwrap_or("");
        let frame2 = frames.get(2).copied().unwrap_or("");
        assert_ne!(frame0, frame2, "the spinner must move between tick 0 and tick 8");

        let mut loading = with_resume(vec![]);
        {
            let Some(ActiveModal::SessionPicker {
                loading, entries, ..
            }) = &mut loading.active_modal
            else {
                panic!("expected session picker");
            };
            *loading = true;
            *entries = None;
        }
        let text = render_resume_text(&mut loading, &theme, 0);
        assert!(
            text.contains(&format!("{frame0} Loading\u{2026}")),
            "a loading resume paints the full-picker spinner:\n{text}"
        );
        assert!(
            !text.contains(&format!("{frame2} Loading\u{2026}")),
            "tick 0 must not show the later spinner frame:\n{text}"
        );
        assert!(
            !text.contains("No sessions"),
            "loading is not an empty directory:\n{text}"
        );
        assert!(
            !text.contains("No matches"),
            "loading is not a search miss:\n{text}"
        );
        let later = render_resume_text(&mut loading, &theme, 8);
        assert!(
            later.contains(&format!("{frame2} Loading\u{2026}")),
            "the spinner advances with the same tick as the full picker:\n{later}"
        );

        let mut empty = with_resume(vec![]);
        let text = render_resume_text(&mut empty, &theme, 0);
        assert!(
            text.contains("No sessions in this directory"),
            "an empty directory uses the full picker's empty line:\n{text}"
        );
        assert!(
            !text.contains("Loading\u{2026}"),
            "a settled empty directory is not still loading:\n{text}"
        );
        assert!(
            !text.contains("No matches"),
            "an empty directory is not a search miss:\n{text}"
        );

        let mut missed = with_resume(vec![session_entry("alpha-session")]);
        {
            let Some(ActiveModal::SessionPicker { state, .. }) = &mut missed.active_modal else {
                panic!("expected session picker");
            };
            state.set_query("zzzz-no-such-session");
        }
        let text = render_resume_text(&mut missed, &theme, 0);
        assert!(
            text.contains("No matches"),
            "a local filter over real rows stays a miss:\n{text}"
        );
        assert!(
            !text.contains("No sessions in this directory"),
            "a filtered list must not claim the directory is empty:\n{text}"
        );

        let mut searched = with_resume(vec![]);
        {
            let Some(ActiveModal::SessionPicker { entries_query, .. }) = &mut searched.active_modal
            else {
                panic!("expected session picker");
            };
            *entries_query = Some("roadmap".into());
        }
        let text = render_resume_text(&mut searched, &theme, 0);
        assert!(
            text.contains("No matches"),
            "a server search that returned nothing stays a miss:\n{text}"
        );
        assert!(
            !text.contains("No sessions in this directory"),
            "a server search must not claim the directory is empty:\n{text}"
        );
    }

    /// A server search matches message text as well as titles. Minimal `/resume` keeps those
    /// rows when the stamp matches the live query, which is the query the keys already use (`v88-x39`).
    #[test]
    fn minimal_resume_keeps_message_text_matches_for_the_key_query() {
        let theme = Theme::groknight();
        let mut titled = session_entry("session-title");
        titled.summary = "hit the deadline".into();
        let mut message = session_entry("session-message");
        message.summary = "Quarterly roadmap notes".into();

        let mut local = with_resume(vec![titled.clone(), message.clone()]);
        {
            let Some(ActiveModal::SessionPicker { state, .. }) = &mut local.active_modal else {
                panic!("expected session picker");
            };
            state.set_query("hit");
        }
        let text = render_resume_text(&mut local, &theme, 0);
        assert!(
            text.contains("hit the deadline"),
            "a title match stays visible:\n{text}"
        );
        assert!(
            !text.contains("Quarterly roadmap notes"),
            "without a server stamp the local filter still hides an unrelated title:\n{text}"
        );

        let mut server = with_resume(vec![titled.clone(), message.clone()]);
        {
            let Some(ActiveModal::SessionPicker {
                state,
                entries_query,
                ..
            }) = &mut server.active_modal
            else {
                panic!("expected session picker");
            };
            state.set_query("hit");
            *entries_query = Some("hit".into());
        }
        let text = render_resume_text(&mut server, &theme, 0);
        assert!(
            text.contains("hit the deadline"),
            "the title match stays with the server stamp:\n{text}"
        );
        assert!(
            text.contains("Quarterly roadmap notes"),
            "a message-text match stays visible for the query the keys use:\n{text}"
        );
        assert!(
            panel_height(&server, ListPanel::Resume, 80, 40)
                > panel_height(&local, ListPanel::Resume, 80, 40),
            "the extra message-text row takes a line in the panel"
        );

        let mut stale = with_resume(vec![titled, message]);
        {
            let Some(ActiveModal::SessionPicker {
                state,
                entries_query,
                ..
            }) = &mut stale.active_modal
            else {
                panic!("expected session picker");
            };
            state.set_query("hit");
            *entries_query = Some("older query".into());
        }
        let text = render_resume_text(&mut stale, &theme, 0);
        assert!(
            text.contains("hit the deadline"),
            "a stale stamp still shows title matches:\n{text}"
        );
        assert!(
            !text.contains("Quarterly roadmap notes"),
            "a stale server stamp must not keep an unrelated title:\n{text}"
        );
    }

    /// The resume search row names the source filter when the row is wide enough, and drops the chip before it covers the query (`v88-x41`).
    #[test]
    fn minimal_resume_search_names_the_source_filter_when_it_fits() {
        let theme = Theme::groknight();

        let mut wide = with_resume(vec![session_entry("alpha-session")]);
        {
            let Some(ActiveModal::SessionPicker {
                source_filter,
                state,
                ..
            }) = &mut wide.active_modal
            else {
                panic!("expected session picker");
            };
            *source_filter = xai_grok_pager::views::session_picker::SourceFilter::External;
            state.search_active = true;
            state.set_query("alpha-session-query");
        }
        let text = render_resume_text(&mut wide, &theme, 0);
        assert!(
            text.contains("External"),
            "a wide row names the active source filter:\n{text}"
        );
        assert!(
            text.contains("alpha-session-query"),
            "the chip must leave the query visible:\n{text}"
        );
        let Some(ActiveModal::SessionPicker { state, .. }) = &wide.active_modal else {
            panic!("expected session picker");
        };
        assert!(
            state.filter_area.is_some(),
            "a wide row keeps the chip as a hit target"
        );

        let mut grok = with_resume(vec![session_entry("alpha-session")]);
        {
            let Some(ActiveModal::SessionPicker { state, .. }) = &mut grok.active_modal else {
                panic!("expected session picker");
            };
            state.search_active = true;
        }
        let text = render_resume_text(&mut grok, &theme, 0);
        assert!(
            text.contains("Grok"),
            "the default filter is named when the row has room:\n{text}"
        );

        let mut narrow = with_resume(vec![session_entry("alpha-session")]);
        {
            let Some(ActiveModal::SessionPicker {
                source_filter,
                state,
                ..
            }) = &mut narrow.active_modal
            else {
                panic!("expected session picker");
            };
            *source_filter = xai_grok_pager::views::session_picker::SourceFilter::External;
            state.search_active = true;
            state.set_query("project-alpha");
        }
        let area = Rect::new(0, 0, 16, 8);
        let mut buf = Buffer::empty(area);
        render_with_tick(&mut buf, area, &mut narrow, ListPanel::Resume, &theme, 0);
        let text = crate::buffer_text(&buf);
        assert!(
            text.contains("alpha"),
            "the narrow query tail stays visible:\n{text}"
        );
        assert!(
            !text.contains("External"),
            "a narrow row omits the chip instead of covering the query:\n{text}"
        );
        let Some(ActiveModal::SessionPicker { state, .. }) = &narrow.active_modal else {
            panic!("expected session picker");
        };
        assert!(
            state.filter_area.is_none(),
            "a dropped chip is not a hit target"
        );

        let mut chat = with_resume(vec![session_entry("alpha-session")]);
        chat.app_chat_mode = true;
        {
            let Some(ActiveModal::SessionPicker { source_filter, .. }) = &mut chat.active_modal
            else {
                panic!("expected session picker");
            };
            *source_filter = xai_grok_pager::views::session_picker::SourceFilter::External;
        }
        let text = render_resume_text(&mut chat, &theme, 0);
        assert!(
            !text.contains("External"),
            "chat mode has no source filter to name:\n{text}"
        );
    }

    /// Grouping off draws the sessions in list order with no repo headers (`v88-x45`).
    #[test]
    fn minimal_resume_flat_list_omits_repo_headers() {
        let theme = Theme::groknight();
        let mut zeta = session_entry("zeta-task");
        zeta.summary = "zeta task".into();
        zeta.repo_name = "zeta-repo".into();
        let mut alpha = session_entry("alpha-task");
        alpha.summary = "alpha task".into();
        alpha.repo_name = "alpha-repo".into();

        let mut flat = with_resume(vec![zeta.clone(), alpha.clone()]);
        {
            let Some(ActiveModal::SessionPicker { grouped, .. }) = &mut flat.active_modal else {
                panic!("expected session picker");
            };
            *grouped = false;
        }
        let flat_text = render_resume_text(&mut flat, &theme, 0);
        let zeta_at = flat_text.find("zeta task").expect("zeta row");
        let alpha_at = flat_text.find("alpha task").expect("alpha row");
        assert!(
            zeta_at < alpha_at,
            "a flat list keeps caller order:\n{flat_text}"
        );
        assert!(
            !flat_text.contains("zeta-repo") && !flat_text.contains("alpha-repo"),
            "a flat list has no repo headers:\n{flat_text}"
        );

        let mut grouped = with_resume(vec![zeta, alpha]);
        let grouped_text = render_resume_text(&mut grouped, &theme, 0);
        assert!(
            grouped_text.contains("alpha-repo") && grouped_text.contains("zeta-repo"),
            "grouping still paints repo headers:\n{grouped_text}"
        );
        let alpha_at = grouped_text.find("alpha task").expect("alpha row");
        let zeta_at = grouped_text.find("zeta task").expect("zeta row");
        assert!(
            alpha_at < zeta_at,
            "grouped rows follow repo order, not caller order:\n{grouped_text}"
        );
    }

    /// An expanded card whose CWD wraps still shows the fields under it (`v88-x52`).
    #[test]
    fn minimal_resume_shows_fields_when_cwd_wraps() {
        let theme = Theme::groknight();
        let mut entry = session_entry("wrap-session");
        entry.cwd = format!("/home/dev/{}", "a".repeat(48)) + "/wrap-tail-dir";
        entry.model_id = Some("grok-4".into());
        let mut agent = with_resume(vec![entry]);
        {
            let Some(ActiveModal::SessionPicker { state, .. }) = &mut agent.active_modal else {
                panic!("expected session picker");
            };
            state.expanded.insert(0);
        }
        let width = 36u16;
        let height = panel_height(&agent, ListPanel::Resume, width, 40);
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render_with_tick(&mut buf, area, &mut agent, ListPanel::Resume, &theme, 0);
        let text = crate::buffer_text(&buf);
        assert!(
            text.contains("wrap-tail-dir"),
            "the wrapped cwd stays on the card:\n{text}"
        );
        assert!(
            text.contains("grok-4"),
            "the model field stays visible under a wrapped cwd:\n{text}"
        );
        assert!(
            text.contains("Source"),
            "fields after the wrapped cwd stay visible:\n{text}"
        );
    }

    /// An expanded card whose CWD has an early space keeps the tail and later fields (`v88-x58`).
    #[test]
    fn minimal_resume_keeps_a_cwd_with_an_early_space() {
        let theme = Theme::groknight();
        let mut entry = session_entry("space-session");
        entry.cwd = format!("/home/dev/My Documents/{}", "early-tail");
        entry.model_id = Some("grok-4".into());
        let mut agent = with_resume(vec![entry]);
        {
            let Some(ActiveModal::SessionPicker { state, .. }) = &mut agent.active_modal else {
                panic!("expected session picker");
            };
            state.expanded.insert(0);
        }
        let width = 36u16;
        let height = panel_height(&agent, ListPanel::Resume, width, 40);
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render_with_tick(&mut buf, area, &mut agent, ListPanel::Resume, &theme, 0);
        let text = crate::buffer_text(&buf);
        let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
        assert!(
            compact.contains("early-tail"),
            "the cwd tail stays on the resume card:\n{text}"
        );
        assert!(
            text.contains("grok-4"),
            "the model field stays visible under a cwd that breaks on a space:\n{text}"
        );
        assert!(
            text.contains("Source"),
            "fields after that cwd stay visible:\n{text}"
        );
    }

    fn render_resume_text(agent: &mut AgentView, theme: &Theme, tick: u64) -> String {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_with_tick(&mut buf, area, agent, ListPanel::Resume, theme, tick);
        crate::buffer_text(&buf)
    }

    #[test]
    fn resume_search_uses_picker_grapheme_viewport_at_narrow_width() {
        let grapheme = "👩🏽\u{200d}💻";
        let combining = "e\u{301}";
        let mut agent = with_resume(vec![session_entry("match")]);
        let Some(ActiveModal::SessionPicker { state, .. }) = &mut agent.active_modal else {
            panic!("expected session picker");
        };
        state.set_query(format!("a{grapheme}{combining}"));
        state.search_active = true;

        let theme = Theme::current();
        let area = Rect::new(0, 0, 14, 5);
        let mut actual = Buffer::empty(area);
        render(&mut actual, area, &mut agent, ListPanel::Resume, &theme);

        let Some(ActiveModal::SessionPicker { state, .. }) = &agent.active_modal else {
            panic!("expected session picker");
        };
        let mut expected = Buffer::empty(area);
        let _ = minimal_api::render_resume_search_row(
            &mut expected,
            Rect::new(1, 1, 13, 1),
            &theme,
            state,
            Some((
                xai_grok_pager::views::session_picker::SourceFilter::Grok.label(),
                "f",
                false,
            )),
        );
        for x in 1..14 {
            let actual_cell = actual.cell((x, 1)).expect("actual search cell");
            let expected_cell = expected.cell((x, 1)).expect("expected search cell");
            assert_eq!(actual_cell.symbol(), expected_cell.symbol(), "column {x}");
            assert_eq!(actual_cell.style(), expected_cell.style(), "column {x}");
        }
        let text = crate::buffer_text(&actual);
        assert!(text.contains(grapheme), "ZWJ grapheme was split: {text:?}");
        assert!(
            text.contains(combining),
            "combining grapheme was split: {text:?}"
        );
        assert_eq!(
            actual.cell((13, 1)).expect("cursor cell").bg,
            theme.text_primary
        );
    }

    #[test]
    fn mcps_panel_height_is_chrome_plus_rows() {
        // One section header + 2 server rows + a tool-summary line under each server.
        // Body is 5; + 4 chrome = 9.
        let a = with_mcps(vec![
            mcp_server("alpha", McpServerDisplayStatus::Ready, 1),
            mcp_server("bravo", McpServerDisplayStatus::Ready, 2),
        ]);
        assert_eq!(panel_height(&a, ListPanel::Mcps, 80, 40), 9);
        // Clamps to the screen ceiling when content is taller.
        assert_eq!(panel_height(&a, ListPanel::Mcps, 80, 5), 5);
    }

    fn with_mcps_data(data: TabDataState<Vec<McpServerInfo>>) -> AgentView {
        let mut a = agent();
        minimal_api::set_extensions_modal(
            &mut a,
            Some(ExtensionsModalState {
                active_tab: ExtensionsTab::McpServers,
                mcps_data: data,
                ..Default::default()
            }),
        );
        a
    }

    fn render_mcps_text(agent: &mut AgentView, theme: &Theme, tick: u64) -> String {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_with_tick(&mut buf, area, agent, ListPanel::Mcps, theme, tick);
        crate::buffer_text(&buf)
    }

    /// Loading, a failed fetch, and the list rows use the full `/mcps` view's
    /// copy and row shape: spinner, `Error: …`, section copy, and badges.
    #[test]
    fn minimal_mcps_loading_error_and_layout_match_the_full_view() {
        let theme = Theme::groknight();

        let frames = xai_grok_pager::glyphs::dot_spinner_frames();
        let frame0 = frames.first().copied().unwrap_or("");
        let frame2 = frames.get(2).copied().unwrap_or("");
        assert_ne!(frame0, frame2, "the spinner must move between tick 0 and tick 8");

        let mut loading = with_mcps_data(TabDataState::Loading);
        let text = render_mcps_text(&mut loading, &theme, 0);
        assert!(
            text.contains(&format!("{frame0} Loading\u{2026}")),
            "loading paints the full-view spinner line:\n{text}"
        );
        assert!(
            !text.contains(&format!("{frame2} Loading\u{2026}")),
            "tick 0 must not show the later spinner frame:\n{text}"
        );
        assert!(
            !text.contains("loading\u{2026}"),
            "the subtitle must not use a lowercase loading line:\n{text}"
        );
        assert!(
            !text.contains("No matches"),
            "loading is not an empty list:\n{text}"
        );
        let later = render_mcps_text(&mut loading, &theme, 8);
        assert!(
            later.contains(&format!("{frame2} Loading\u{2026}")),
            "the spinner advances with the same tick as the full view:\n{later}"
        );

        let mut failed = with_mcps_data(TabDataState::Error("the catalog failed".into()));
        let text = render_mcps_text(&mut failed, &theme, 0);
        assert!(
            text.contains("Error: the catalog failed"),
            "a failed fetch is a row, as in the full view:\n{text}"
        );
        assert!(
            !text.contains("error: the catalog failed"),
            "the error must not sit in a lowercase subtitle:\n{text}"
        );
        assert!(
            !text.contains("No matches"),
            "an error is not an empty list:\n{text}"
        );

        let mut linear = mcp_server("linear", McpServerDisplayStatus::Ready, 1);
        linear.wire_source = McpWireSource::Managed;
        linear.source = "managed".into();
        linear.tools = vec![McpToolDetail {
            name: "create_issue".into(),
            display_name: None,
            description: Some("Creates an issue".into()),
            enabled: false,
        }];
        let mut plugin_srv = mcp_server("hidden-plugin-srv", McpServerDisplayStatus::Ready, 0);
        plugin_srv.plugin_name = Some("acme".into());
        plugin_srv.source = "plugin: acme".into();
        let mut disabled = mcp_server("off-srv", McpServerDisplayStatus::Unavailable, 0);
        disabled.enabled = false;
        let mut agent = with_mcps(vec![linear, plugin_srv, disabled]);
        {
            let state = minimal_api::extensions_modal_mut(&mut agent).unwrap();
            state.session_team_id = Some("team-1".into());
            state.mcps_tools_expanded.insert(0);
            // Section, linear, create_issue, then the other section headers.
            state.picker_state.expanded.insert(2);
        }
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render(&mut buf, area, &mut agent, ListPanel::Mcps, &theme);
        let text = crate::buffer_text(&buf);
        assert!(
            text.contains("Add, remove, or manage connectors. Ctrl+O to open or go to:"),
            "the managed section keeps the full-view description:\n{text}"
        );
        assert!(
            text.contains("grok.com/connectors?teamId=team-1"),
            "the connectors link uses the session team:\n{text}"
        );
        let url_underlined = (0..area.height).any(|y| {
            let line_has_url = crate::buffer_text(&buf)
                .lines()
                .nth(y as usize)
                .is_some_and(|line| line.contains("grok.com/connectors?teamId=team-1"));
            line_has_url
                && (0..area.width).any(|x| {
                    buf.cell((x, y)).is_some_and(|cell| {
                        cell.modifier.contains(Modifier::UNDERLINED) && cell.symbol() != " "
                    })
                })
        });
        assert!(
            url_underlined,
            "the connectors url is underlined like the full view:\n{text}"
        );
        assert!(
            text.contains("[ready]"),
            "a live server uses the full-view bracketed status:\n{text}"
        );
        assert!(
            text.contains("[disabled]"),
            "a disabled tool uses the full-view badge:\n{text}"
        );
        assert!(
            text.contains("Creates an issue"),
            "an opened tool shows its description:\n{text}"
        );
        assert!(
            text.contains("Plugin: acme"),
            "a plugin section header is present:\n{text}"
        );
        assert!(
            !text.contains("hidden-plugin-srv"),
            "plugin sections start collapsed, as in the full view:\n{text}"
        );
        assert!(
            text.contains("off-srv"),
            "a disabled local server stays listed:\n{text}"
        );

        let off_y = (0..area.height).find(|y| {
            (0..area.width).any(|x| {
                buf.cell((x, *y))
                    .is_some_and(|cell| cell.symbol() == "o" || cell.symbol() == "off-srv")
            }) && crate::buffer_text(&buf)
                .lines()
                .nth(*y as usize)
                .is_some_and(|line| line.contains("off-srv"))
        });
        let off_y = off_y.expect("off-srv row");
        let name_cell = (0..area.width).find_map(|x| {
            let cell = buf.cell((x, off_y))?;
            (cell.symbol() == "o").then_some(cell)
        });
        let name_cell = name_cell.expect("off-srv label cell");
        assert_eq!(
            name_cell.fg, theme.gray_dim,
            "a disabled server is dimmed like the full list"
        );

        let mut managed_only = mcp_server("linear", McpServerDisplayStatus::Ready, 0);
        managed_only.wire_source = McpWireSource::Managed;
        assert_eq!(
            panel_height(&with_mcps(vec![managed_only]), ListPanel::Mcps, 80, 40),
            9,
            "the managed section's two description rows count toward the panel height"
        );
    }

    fn render_mcps_wide(agent: &mut AgentView, theme: &Theme) -> String {
        let area = Rect::new(0, 0, 120, 24);
        let mut buf = Buffer::empty(area);
        render(&mut buf, area, agent, ListPanel::Mcps, theme);
        crate::buffer_text(&buf)
    }

    /// The minimal `/mcps` row under the title is the full view's search bar, the
    /// footer lists that view's action hints, and `a` / `i` open the add and setup forms.
    #[test]
    fn minimal_mcps_search_hints_and_forms_match_the_full_view() {
        let theme = Theme::groknight();
        let mut listed = with_mcps(vec![
            mcp_server("alpha", McpServerDisplayStatus::Ready, 1),
            mcp_server("bravo", McpServerDisplayStatus::Ready, 1),
        ]);
        let text = render_mcps_wide(&mut listed, &theme);
        assert!(
            text.contains("/ to search"),
            "the search row shows the full-view hint:\n{text}"
        );
        assert!(text.contains("All"), "the filter sits on the search row:\n{text}");
        for hint in [
            "Tab tabs",
            "r refresh",
            "a add",
            "i auth",
            "Space enable/disable",
            "x remove",
            "Ctrl+O open",
            "Esc close",
        ] {
            assert!(text.contains(hint), "missing {hint}:\n{text}");
        }

        minimal_api::press_extensions_modal_key(&mut listed, '/');
        minimal_api::press_extensions_modal_key(&mut listed, 'z');
        let text = render_mcps_wide(&mut listed, &theme);
        assert!(
            text.contains("search:"),
            "a typed query uses the search bar:\n{text}"
        );
        assert!(text.contains('z'), "the query is painted:\n{text}");
        assert!(
            !text.contains("alpha"),
            "the query filters the server list:\n{text}"
        );

        let mut adding = with_mcps(vec![mcp_server(
            "alpha",
            McpServerDisplayStatus::Ready,
            1,
        )]);
        minimal_api::press_extensions_modal_key(&mut adding, 'a');
        let text = render_mcps_wide(&mut adding, &theme);
        assert!(
            text.contains("URL / Command"),
            "a opens the add form:\n{text}"
        );
        assert!(
            text.contains("Enter submit"),
            "the add form uses the full-view footer:\n{text}"
        );
        assert!(
            text.contains("Esc cancel"),
            "the add form can be cancelled:\n{text}"
        );
        assert!(
            !text.contains("alpha"),
            "the add form replaces the server list:\n{text}"
        );

        let mut setup_srv = mcp_server("needs-setup", McpServerDisplayStatus::NeedsAuth, 0);
        setup_srv.setup_required = true;
        setup_srv.setup = Some(McpSetupConfig {
            fields: vec![McpSetupField {
                id: "region".into(),
                label: "Region".into(),
                field_type: "select".into(),
                required: true,
                default: None,
                options: vec![McpSetupOption {
                    label: "us-east".into(),
                    value: "us-east".into(),
                }],
            }],
        });
        let mut setup = with_mcps(vec![setup_srv]);
        render_mcps_wide(&mut setup, &theme);
        {
            let state = minimal_api::extensions_modal_mut(&mut setup).unwrap();
            state.picker_state.selected = 1;
        }
        minimal_api::press_extensions_modal_key(&mut setup, 'i');
        let text = render_mcps_wide(&mut setup, &theme);
        assert!(
            text.contains("Save and authenticate"),
            "i on a setup server opens the setup form:\n{text}"
        );
        assert!(
            text.contains("us-east"),
            "the setup form lists its choices:\n{text}"
        );
        assert!(
            text.contains("Enter save and authenticate"),
            "the setup form uses the full-view footer:\n{text}"
        );
    }
}
