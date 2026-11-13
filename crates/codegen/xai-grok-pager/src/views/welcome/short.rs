//! Welcome screens that are one or two rows tall, and the resume list below eight columns.
//!
//! The stacked solver spends its rows on a logo, a top bar, and a three-row prompt. On a
//! one- or two-row terminal that leaves the action with no cell, so this path paints
//! into the rows that exist.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::{WelcomeRenderParams, WelcomeRenderResult, session_menu_rows};
use crate::app::app_view::{AuthMode, AuthState, TrustState};
use crate::app::consent::{ConsentLegibility, ConsentState};
use crate::render::SafeBuf;
use crate::theme::Theme;
use crate::views::picker::PickerState;
use crate::views::prompt_widget::PromptWidget;

pub(super) fn render_welcome_short(
    area: Rect,
    buf: &mut Buffer,
    params: &WelcomeRenderParams<'_>,
    prompt: &PromptWidget,
    picker: &PickerState,
) -> WelcomeRenderResult {
    let theme = Theme::current();
    if area.height == 0 || area.width == 0 {
        return finish(WelcomeRenderResult::default());
    }
    buf.set_style(area, Style::default().bg(theme.bg_base));

    let result = match params.auth_state {
        AuthState::Pending { error } => paint_pending(area, buf, &theme, params, error.as_deref()),
        AuthState::Authenticating { auth_url, mode, .. } => {
            paint_authenticating(area, buf, &theme, *mode, auth_url.as_deref())
        }
        AuthState::Done if params.is_zdr_blocked => paint_zdr(area, buf, &theme, params),
        AuthState::Done if params.has_access => {
            if let ConsentState::Pending { .. } = params.consent_state {
                paint_consent(area, buf, &theme, params)
            } else if let TrustState::Pending { .. } = params.trust_state {
                paint_trust(area, buf, &theme, params)
            } else if params.session_picker.is_some() || params.session_picker_loading {
                paint_short_picker(area, buf, &theme, params, picker)
            } else {
                paint_home(area, buf, &theme, params, prompt)
            }
        }
        AuthState::Done => paint_gate(area, buf, &theme, params),
    };
    finish(result)
}

/// Resume summaries when the bordered picker frame does not fit (it needs ten columns).
pub(super) fn paint_narrow_session_list(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
    picker: &PickerState,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let style = Style::default().fg(theme.text_primary);
    paint_line(buf, area.x, area.y, area.width, "Resume session", style);
    if area.height < 2 {
        return;
    }
    let mut y = area.y.saturating_add(1);
    let bottom = area.y.saturating_add(area.height);
    if params.session_picker_loading {
        paint_line(buf, area.x, y, area.width, "Loading…", style);
        return;
    }
    let summaries = session_summaries(params, picker);
    if summaries.is_empty() {
        paint_line(buf, area.x, y, area.width, "No sessions", style);
        return;
    }
    let selected = picker.selected.min(summaries.len() - 1);
    let room = (bottom.saturating_sub(y)) as usize;
    if room == 0 {
        return;
    }
    let start = if summaries.len() <= room {
        0
    } else {
        selected
            .saturating_sub(room / 2)
            .min(summaries.len() - room)
    };
    for index in start..summaries.len().min(start.saturating_add(room)) {
        if y >= bottom {
            break;
        }
        let row_style = if index == selected {
            buf.set_style(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                Style::default().bg(theme.bg_highlight),
            );
            style.bg(theme.bg_highlight).add_modifier(Modifier::BOLD)
        } else {
            style
        };
        let line = summaries.get(index).map(String::as_str).unwrap_or("");
        paint_line(buf, area.x, y, area.width, line, row_style);
        y = y.saturating_add(1);
    }
}

fn finish(mut result: WelcomeRenderResult) -> WelcomeRenderResult {
    if result.post_flush_escapes.is_none() {
        result.post_flush_escapes = crate::terminal::overlay::clear().map(Into::into);
    }
    result
}

fn quit_key() -> &'static str {
    if super::welcome_in_vscode_family() {
        "Ctrl+D"
    } else {
        "Ctrl+Q"
    }
}

fn quit_hint() -> String {
    format!("{}  quit", quit_key())
}

fn cols(text: &str) -> u16 {
    UnicodeWidthStr::width(text) as u16
}

/// Clip to `width` columns. A one-column budget keeps the first character,
/// so the row is not only an ellipsis.
fn clip_to_cols(text: &str, width: u16) -> String {
    if width == 0 || text.is_empty() {
        return String::new();
    }
    if cols(text) <= width {
        return text.to_string();
    }
    if width == 1 {
        return text
            .chars()
            .next()
            .map(|ch| ch.to_string())
            .unwrap_or_default();
    }
    crate::render::line_utils::truncate_str(text, width as usize)
}

fn paint_line(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let clipped = clip_to_cols(text, width);
    if clipped.is_empty() {
        return;
    }
    let drawn = cols(&clipped).min(width);
    buf.set_span_safe(x, y, &Span::styled(clipped, style), drawn);
}

fn status_style(theme: &Theme) -> Style {
    Style::default().fg(theme.gray_bright)
}

fn action_style(theme: &Theme, selected: bool) -> Style {
    let style = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD);
    if selected {
        style.bg(theme.bg_highlight)
    } else {
        style
    }
}

/// Status on the first row when the terminal has two. The action row is always the last one.
fn split_short(area: Rect) -> (Option<Rect>, Rect) {
    if area.height >= 2 {
        let status = Rect { height: 1, ..area };
        let action = Rect {
            y: area.y.saturating_add(1),
            height: 1,
            ..area
        };
        (Some(status), action)
    } else {
        (None, Rect { height: 1, ..area })
    }
}

fn paint_status(buf: &mut Buffer, row: Option<Rect>, text: &str, theme: &Theme) {
    let Some(row) = row else {
        return;
    };
    paint_line(buf, row.x, row.y, row.width, text, status_style(theme));
}

/// Pack labels left to right. `reserve_last` keeps the final label on the
/// right when one column remains for the rest.
fn paint_action_row(
    row: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    items: &[(&str, &str)],
    selected: Option<usize>,
    reserve_last: bool,
) -> Vec<Rect> {
    let mut rects = vec![Rect::default(); items.len()];
    if items.is_empty() || row.height == 0 || row.width == 0 {
        return rects;
    }
    let mut prefix_width = row.width;
    let mut parked = false;
    if reserve_last && items.len() >= 2 {
        let last = items[items.len() - 1].1;
        let last_w = cols(last);
        if last_w > 0 && last_w.saturating_add(1) < row.width {
            parked = true;
            prefix_width = row.width.saturating_sub(last_w.saturating_add(1));
            let x = row.x.saturating_add(row.width.saturating_sub(last_w));
            let drawn = paint_action_label(
                buf,
                theme,
                x,
                row.y,
                last_w,
                last,
                selected == Some(items.len() - 1),
            );
            if drawn > 0 {
                rects[items.len() - 1] = Rect {
                    x,
                    y: row.y,
                    width: drawn,
                    height: 1,
                };
            }
        }
    }
    let prefix_len = if parked { items.len() - 1 } else { items.len() };
    let mut x = row.x;
    let right = row.x.saturating_add(prefix_width);
    for i in 0..prefix_len {
        if x >= right {
            break;
        }
        let available = right.saturating_sub(x);
        let label = items[i].1;
        let full = cols(label);
        if full == 0 || available == 0 {
            break;
        }
        let more = i + 1 < prefix_len;
        let fits = if more {
            full.saturating_add(1) <= available
        } else {
            full <= available
        };
        let budget = if fits { full } else { available };
        let drawn = paint_action_label(buf, theme, x, row.y, budget, label, selected == Some(i));
        if drawn > 0 {
            rects[i] = Rect {
                x,
                y: row.y,
                width: drawn,
                height: 1,
            };
        }
        if !fits {
            break;
        }
        x = x.saturating_add(full);
        if more {
            x = x.saturating_add(1);
        }
    }
    rects
}

fn paint_action_label(
    buf: &mut Buffer,
    theme: &Theme,
    x: u16,
    y: u16,
    width: u16,
    text: &str,
    selected: bool,
) -> u16 {
    let clipped = clip_to_cols(text, width);
    let drawn = cols(&clipped);
    if drawn == 0 {
        return 0;
    }
    if selected {
        buf.set_style(
            Rect {
                x,
                y,
                width: drawn,
                height: 1,
            },
            Style::default().bg(theme.bg_highlight),
        );
    }
    buf.set_span_safe(
        x,
        y,
        &Span::styled(clipped, action_style(theme, selected)),
        drawn,
    );
    drawn
}

fn paint_pending(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
    error: Option<&str>,
) -> WelcomeRenderResult {
    let label = params.login_label.unwrap_or("grok.com");
    let login_text = format!("Login with {label}");
    let items = [("l", login_text.as_str()), ("q", "Quit")];
    let (status, action) = split_short(area);
    paint_status(buf, status, error.unwrap_or(login_text.as_str()), theme);
    WelcomeRenderResult {
        menu_rects: paint_action_row(action, buf, theme, &items, params.selected, true),
        ..Default::default()
    }
}

fn paint_authenticating(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    mode: AuthMode,
    auth_url: Option<&str>,
) -> WelcomeRenderResult {
    let status_text = match mode {
        AuthMode::Pending => "Connecting...".to_string(),
        AuthMode::Device => super::extract_user_code(auth_url.unwrap_or(""))
            .map(str::to_string)
            .unwrap_or_else(|| "Waiting for approval...".to_string()),
        AuthMode::Command | AuthMode::Loopback => "Waiting for login to complete...".to_string(),
    };
    let hint = quit_hint();
    let (status, action) = split_short(area);
    if status.is_some() {
        paint_status(buf, status, &status_text, theme);
        paint_line(
            buf,
            action.x,
            action.y,
            action.width,
            &hint,
            action_style(theme, false),
        );
    } else if cols(&status_text)
        .saturating_add(1)
        .saturating_add(cols(&hint))
        <= area.width
    {
        paint_line(
            buf,
            area.x,
            area.y,
            cols(&status_text),
            &status_text,
            status_style(theme),
        );
        let x = area.x.saturating_add(cols(&status_text)).saturating_add(1);
        paint_line(
            buf,
            x,
            area.y,
            area.width.saturating_sub(x.saturating_sub(area.x)),
            &hint,
            action_style(theme, false),
        );
    } else {
        paint_line(
            buf,
            action.x,
            action.y,
            action.width,
            &hint,
            action_style(theme, false),
        );
    }
    // Quit is Esc / Ctrl+Q on the app handler. This screen has no menu index.
    WelcomeRenderResult::default()
}

fn paint_zdr(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
) -> WelcomeRenderResult {
    let items = [("l", "Switch account"), ("q", "Quit")];
    let (status, action) = split_short(area);
    paint_status(
        buf,
        status,
        "Grok Build is not yet available for this account.",
        theme,
    );
    WelcomeRenderResult {
        menu_rects: paint_action_row(action, buf, theme, &items, params.selected, true),
        ..Default::default()
    }
}

fn paint_consent(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
) -> WelcomeRenderResult {
    let items = [("q", "Quit")];
    let (status, action) = split_short(area);
    paint_status(buf, status, "Window too small", theme);
    WelcomeRenderResult {
        menu_rects: paint_action_row(action, buf, theme, &items, params.selected, false),
        consent_legibility: Some(ConsentLegibility::Illegible),
        ..Default::default()
    }
}

fn paint_trust(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
) -> WelcomeRenderResult {
    let items = [("y", "Yes, proceed"), ("n", "No, quit")];
    let (status, action) = split_short(area);
    paint_status(
        buf,
        status,
        "Do you trust the contents of this directory?",
        theme,
    );
    WelcomeRenderResult {
        menu_rects: paint_action_row(action, buf, theme, &items, params.selected, true),
        ..Default::default()
    }
}

fn paint_gate(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
) -> WelcomeRenderResult {
    let cta = params
        .gate
        .and_then(|gate| gate.label.clone())
        .unwrap_or_else(|| "Upgrade Subscription".to_string());
    let items = [
        ("Ctrl+G", cta.as_str()),
        ("Ctrl+L", "Logout"),
        (quit_key(), "Quit"),
    ];
    let status_text = match (
        params.gate.map(|gate| gate.message.as_str()),
        params.subscription_tier,
    ) {
        (Some(message), _) if !message.is_empty() => message.to_string(),
        (_, Some(tier)) => format!("Tier: {tier}"),
        _ => "SuperGrok subscription required".to_string(),
    };
    let (status, action) = split_short(area);
    paint_status(buf, status, &status_text, theme);
    WelcomeRenderResult {
        menu_rects: paint_action_row(action, buf, theme, &items, params.selected, true),
        ..Default::default()
    }
}

fn paint_home(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
    prompt: &PromptWidget,
) -> WelcomeRenderResult {
    let rows = session_menu_rows(
        params.has_claude_import,
        crate::app::app_view::worktree_options_hidden(),
        true,
        quit_key(),
    );
    if area.height >= 2 {
        let prompt_row = Rect { height: 1, ..area };
        let action = Rect {
            y: area.y.saturating_add(1),
            height: 1,
            ..area
        };
        super::prompt::paint_prompt_sliver(prompt_row, buf, prompt);
        return WelcomeRenderResult {
            menu_rects: paint_action_row(action, buf, theme, &rows, params.selected, true),
            prompt_rect: Some(prompt_row),
            changelog_action_present: true,
            ..Default::default()
        };
    }
    paint_home_single_row(area, buf, theme, params, prompt, &rows)
}

/// One row holds the prompt on the left and Quit on the right, when both fit.
fn paint_home_single_row(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
    prompt: &PromptWidget,
    rows: &[(&str, &str)],
) -> WelcomeRenderResult {
    let mut menu_rects = vec![Rect::default(); rows.len()];
    let quit_at = rows.iter().position(|(_, label)| *label == "Quit");
    let quit_w = cols("Quit");
    let arrow_w = crate::glyphs::PROMPT_ARROW_WIDTH;
    if quit_w > 0 && area.width >= arrow_w.saturating_add(1).saturating_add(quit_w) {
        let prompt_w = area.width.saturating_sub(quit_w.saturating_add(1));
        let prompt_area = Rect {
            width: prompt_w,
            height: 1,
            ..area
        };
        super::prompt::paint_prompt_sliver(prompt_area, buf, prompt);
        let x = area.x.saturating_add(prompt_w).saturating_add(1);
        let drawn = paint_action_label(
            buf,
            theme,
            x,
            area.y,
            quit_w,
            "Quit",
            quit_at.is_some_and(|index| params.selected == Some(index)),
        );
        if let Some(index) = quit_at
            && drawn > 0
        {
            menu_rects[index] = Rect {
                x,
                y: area.y,
                width: drawn,
                height: 1,
            };
        }
        WelcomeRenderResult {
            menu_rects,
            prompt_rect: Some(prompt_area),
            changelog_action_present: true,
            ..Default::default()
        }
    } else {
        super::prompt::paint_prompt_sliver(area, buf, prompt);
        WelcomeRenderResult {
            menu_rects,
            prompt_rect: Some(Rect { height: 1, ..area }),
            changelog_action_present: true,
            ..Default::default()
        }
    }
}

fn paint_short_picker(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    params: &WelcomeRenderParams<'_>,
    picker: &PickerState,
) -> WelcomeRenderResult {
    let summary = if params.session_picker_loading {
        "Loading…".to_string()
    } else {
        let rows = session_summaries(params, picker);
        if rows.is_empty() {
            "No sessions".to_string()
        } else {
            let index = picker.selected.min(rows.len() - 1);
            rows.get(index).cloned().unwrap_or_default()
        }
    };
    let style = status_style(theme);
    if area.height >= 2 {
        paint_line(buf, area.x, area.y, area.width, "Resume session", style);
        let y = area.y.saturating_add(1);
        paint_line(
            buf,
            area.x,
            y,
            area.width,
            &summary,
            action_style(theme, true),
        );
    } else {
        let title = "Resume session";
        let title_w = cols(title);
        let summary_w = cols(&summary);
        if summary_w > 0 && title_w.saturating_add(1).saturating_add(summary_w) <= area.width {
            paint_line(buf, area.x, area.y, title_w, title, style);
            let x = area.x.saturating_add(title_w).saturating_add(1);
            paint_line(
                buf,
                x,
                area.y,
                area.width.saturating_sub(x.saturating_sub(area.x)),
                &summary,
                action_style(theme, true),
            );
        } else {
            paint_line(buf, area.x, area.y, area.width, title, style);
        }
    }
    WelcomeRenderResult::default()
}

fn session_summaries(params: &WelcomeRenderParams<'_>, picker: &PickerState) -> Vec<String> {
    let Some(entries) = params.session_picker else {
        return Vec::new();
    };
    let query = crate::views::session_picker::effective_filter_query(
        picker.query(),
        params.session_picker_entries_query,
    );
    crate::views::session_picker::filter_session_entries(
        Some(entries),
        query,
        params.session_picker_source_filter,
    )
    .into_iter()
    .filter_map(|index| entries.get(index).map(|entry| entry.summary.clone()))
    .collect()
}
