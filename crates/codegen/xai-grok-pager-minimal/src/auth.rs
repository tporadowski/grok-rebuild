//! Minimal-mode sign-in / folder-trust rendering for the live region.
//!
//! Minimal has no welcome screen, so before any agent session exists the live region shows the sign-in flow itself.
//! [`draw_live`](super::live::draw_live) maps [`AuthState`] and [`TrustState`] to a [`MinimalAuthHint`] and renders it via [`render_auth`].

use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use xai_grok_pager::app::app_view::{AuthState, TrustState};
use xai_grok_pager::theme::Theme;

/// What the minimal live region shows when there is no active agent yet.
/// Computed before the draw closure so the closure can own it.
pub(super) enum MinimalAuthHint {
    /// Interactive sign-in underway: show the URL (when known) and the device code (when the URL carries one).
    /// The code is the line by itself. The caption under it is the same sentence the full sign-in screen uses.
    /// Covers device flow and the external command flow, where the provider opens its own browser and `url` may be `None`.
    SigningIn {
        url: Option<String>,
        code: Option<String>,
    },
    /// The last sign-in attempt failed; show the error.
    Failed(String),
    /// Sign-in has started and the flow (browser, device, or command) is not known yet.
    Connecting,
    /// Authenticated, but the cwd has untrusted repo-local config: ask before creating a session.
    /// Input (y/Enter trust, n/Esc quit) is handled by the welcome interceptor in `AppView::handle_input`; this is render-only.
    TrustFolder { workspace: PathBuf },
    /// Authenticated and trusted; the session is being created (brief transient).
    Starting,
}

/// Map the app's auth and trust state to what the no-agent live region should show.
/// Mirrors the welcome screen's gate order: trust is only offered after auth is `Done`, when the user has access and is not ZDR-blocked.
/// Those gates already block sessions, and the input interceptor only answers trust under the same conditions.
pub(super) fn minimal_auth_hint(
    auth: &AuthState,
    trust: &TrustState,
    has_access: bool,
    is_zdr_blocked: bool,
) -> MinimalAuthHint {
    match auth {
        AuthState::Authenticating {
            mode: xai_grok_pager::app::app_view::AuthMode::Pending,
            ..
        } => MinimalAuthHint::Connecting,
        AuthState::Authenticating { auth_url, mode, .. } => MinimalAuthHint::SigningIn {
            url: auth_url.clone(),
            code: (*mode == xai_grok_pager::app::app_view::AuthMode::Device)
                .then(|| auth_url.as_deref().and_then(device_user_code).map(str::to_owned))
                .flatten(),
        },
        AuthState::Pending { error: Some(err) } => MinimalAuthHint::Failed(err.clone()),
        // Login is starting. The flow type arrives with `Authenticating`.
        AuthState::Pending { error: None } => MinimalAuthHint::Connecting,
        AuthState::Done if has_access && !is_zdr_blocked => {
            if let TrustState::Pending { workspace } = trust {
                MinimalAuthHint::TrustFolder {
                    workspace: workspace.clone(),
                }
            } else {
                MinimalAuthHint::Starting
            }
        }
        AuthState::Done => MinimalAuthHint::Starting,
    }
}

/// Rows the no-agent live region needs for `hint` (before path wrap).
/// Used by the overlay host so the viewport grows enough to show the trust question instead of clipping to the idle prompt height.
pub(super) fn auth_hint_rows(hint: &MinimalAuthHint, width: u16) -> u16 {
    match hint {
        // header + blank + "Opening browser…"
        MinimalAuthHint::SigningIn { url: None, code: _ } => 3,
        // header + blank + "Open this URL" + url rows + optional code block + blank + "Waiting…"
        MinimalAuthHint::SigningIn {
            url: Some(url),
            code,
        } => {
            let url_rows = wrapped_char_rows(url, width);
            // blank + the code alone + the caption under it
            let code_rows = if code.is_some() {
                2 + wrapped_word_rows(DEVICE_CODE_CAPTION, width)
            } else {
                0
            };
            3 + url_rows + code_rows + 2
        }
        // The error message only. A heading would repeat it.
        MinimalAuthHint::Failed(err) => wrapped_word_rows(err, width).max(1),
        MinimalAuthHint::Connecting => 1,
        // question + path rows + blank + 2 warning + blank + 2 menu + blank + hint
        MinimalAuthHint::TrustFolder { workspace } => {
            let path = workspace.display().to_string();
            let path_rows = wrapped_char_rows(&path, width);
            1 + path_rows + 1 + 2 + 1 + 2 + 1 + 1
        }
        MinimalAuthHint::Starting => 1,
    }
}

/// Word-wrap `text` to `width` columns. A single word wider than `width` stays one line.
fn wrap_words(text: &str, width: u16) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;
    let w = width.max(1) as usize;
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.width() + 1 + word.width() <= w {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn wrapped_word_rows(text: &str, width: u16) -> u16 {
    wrap_words(text, width).len() as u16
}

/// How many rows `text` needs when painted char-by-char at `width` (no wrap-inserted spaces); same layout as [`render_url`].
fn wrapped_char_rows(text: &str, width: u16) -> u16 {
    let width = width.max(1) as usize;
    let chars = text.chars().filter(|c| !c.is_control()).count();
    if chars == 0 {
        return 1;
    }
    chars.div_ceil(width) as u16
}

/// Same sentence as the full sign-in screen's device-code caption.
const DEVICE_CODE_CAPTION: &str = "Make sure your browser shows this code.";

/// Parse the device-flow `user_code` from a verification URL (`None` if absent or malformed).
/// Mirrors `views::welcome::extract_user_code`, kept local so minimal does not depend on welcome-screen internals.
fn device_user_code(url: &str) -> Option<&str> {
    let code = url
        .split('?')
        .nth(1)?
        .split('&')
        .find_map(|kv| kv.strip_prefix("user_code="))?;
    (!code.is_empty() && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .then_some(code)
}

/// Write `line` at row `y` (when it fits) and return the next row.
fn put_line(buf: &mut Buffer, area: Rect, y: u16, bottom: u16, line: Line<'_>) -> u16 {
    if y < bottom {
        buf.set_line(area.x, y, &line, area.width);
        y + 1
    } else {
        y
    }
}

/// Write `url` char-by-char across as many rows as it needs (no wrap-inserted spaces), so the terminal's native selection copies it verbatim.
/// Minimal has no mouse capture, so copy is the terminal's job.
/// Returns the next free row.
fn render_url(
    buf: &mut Buffer,
    area: Rect,
    start_y: u16,
    bottom: u16,
    url: &str,
    style: Style,
) -> u16 {
    let width = area.width.max(1);
    // Snapshot the buffer bounds as values so the `&Rect` borrow doesn't outlive the mutable cell writes below
    let (max_x, max_y) = {
        let a = buf.area();
        (a.right(), a.bottom())
    };
    let mut col = 0u16;
    let mut y = start_y;
    for ch in url.chars() {
        // Skip control chars to prevent terminal escape injection.
        if ch.is_control() {
            continue;
        }
        if col >= width {
            col = 0;
            y = y.saturating_add(1);
        }
        if y >= bottom {
            return bottom;
        }
        let x = area.x + col;
        if x < max_x
            && y < max_y
            && let Some(cell) = buf.cell_mut((x, y))
        {
            cell.set_char(ch).set_style(style);
        }
        col += 1;
    }
    y.saturating_add(1)
}

/// Render the sign-in / trust flow (or transient status) in the live region when no agent exists yet.
/// Top-aligned in `area`; clips to its height.
pub(super) fn render_auth(buf: &mut Buffer, area: Rect, theme: &Theme, hint: &MinimalAuthHint) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let bottom = area.y + area.height;
    let mut y = area.y;
    let gray = theme.muted().bg(Color::Reset);
    let bold = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD)
        .bg(Color::Reset);

    match hint {
        MinimalAuthHint::SigningIn { url, code } => {
            y = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(Span::styled("Sign in to Grok", bold)),
            );
            y = put_line(buf, area, y, bottom, Line::default());
            match url {
                Some(url) => {
                    y = put_line(
                        buf,
                        area,
                        y,
                        bottom,
                        Line::from(Span::styled(
                            "Open this URL in your browser to approve:",
                            gray,
                        )),
                    );
                    y = render_url(
                        buf,
                        area,
                        y,
                        bottom,
                        url,
                        Style::default().fg(theme.accent_user).bg(Color::Reset),
                    );
                    if let Some(code) = code {
                        y = put_line(buf, area, y, bottom, Line::default());
                        y = put_line(
                            buf,
                            area,
                            y,
                            bottom,
                            Line::from(Span::styled(code.clone(), bold)),
                        );
                        for line in wrap_words(DEVICE_CODE_CAPTION, area.width) {
                            y = put_line(
                                buf,
                                area,
                                y,
                                bottom,
                                Line::from(Span::styled(line, gray)),
                            );
                        }
                    }
                    y = put_line(buf, area, y, bottom, Line::default());
                    let _ = put_line(
                        buf,
                        area,
                        y,
                        bottom,
                        Line::from(Span::styled("Waiting for approval\u{2026}", gray)),
                    );
                }
                None => {
                    let _ = put_line(
                        buf,
                        area,
                        y,
                        bottom,
                        Line::from(Span::styled(
                            "Opening your browser to sign in\u{2026}",
                            gray,
                        )),
                    );
                }
            }
        }
        MinimalAuthHint::Failed(err) => {
            let failed = Style::default().fg(theme.accent_error).bg(Color::Reset);
            for line in wrap_words(err, area.width) {
                y = put_line(
                    buf,
                    area,
                    y,
                    bottom,
                    Line::from(Span::styled(line, failed)),
                );
            }
        }
        MinimalAuthHint::TrustFolder { workspace } => {
            // Mirrors `render_welcome_trust` copy, flush-left for minimal.
            y = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(Span::styled(
                    "Do you trust the contents of this directory?",
                    bold,
                )),
            );
            y = render_url(
                buf,
                area,
                y,
                bottom,
                &workspace.display().to_string(),
                Style::default().fg(theme.accent_user).bg(Color::Reset),
            );
            y = put_line(buf, area, y, bottom, Line::default());
            y = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(Span::styled(
                    "Grok Build may run or modify contents in this directory,",
                    gray,
                )),
            );
            y = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(Span::styled("posing security risks.", gray)),
            );
            y = put_line(buf, area, y, bottom, Line::default());
            y = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(vec![
                    Span::styled("y", bold),
                    Span::styled(" Yes, proceed", gray),
                ]),
            );
            y = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(vec![
                    Span::styled("n", bold),
                    Span::styled(" No, quit", gray),
                ]),
            );
            y = put_line(buf, area, y, bottom, Line::default());
            let _ = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(Span::styled(
                    "Enter or y to trust \u{00b7} n or Esc to quit",
                    gray,
                )),
            );
        }
        MinimalAuthHint::Connecting => {
            let _ = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(Span::styled("Connecting\u{2026}", gray)),
            );
        }
        MinimalAuthHint::Starting => {
            let _ = put_line(
                buf,
                area,
                y,
                bottom,
                Line::from(Span::styled(
                    "Signing in\u{2026} starting your session.",
                    gray,
                )),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_user_code_parses_verification_url() {
        assert_eq!(
            device_user_code("https://accounts.x.ai/oauth2/device?user_code=ABCD-EFGH"),
            Some("ABCD-EFGH")
        );
        assert_eq!(
            device_user_code("https://accounts.x.ai/oauth2/device"),
            None
        );
        assert_eq!(device_user_code("https://x/device?other=1"), None);
    }

    #[test]
    fn auth_hint_maps_auth_state() {
        use xai_grok_pager::app::app_view::AuthMode;

        let trust_done = TrustState::Done;

        // Device flow maps to SigningIn carrying the URL and the parsed code
        let st = AuthState::Authenticating {
            request_seq: 1,
            handle: None,
            auth_url: Some("https://accounts.x.ai/device?user_code=ABCD-EFGH".into()),
            mode: AuthMode::Device,
        };
        match minimal_auth_hint(&st, &trust_done, true, false) {
            MinimalAuthHint::SigningIn { url, code } => {
                assert_eq!(
                    url.as_deref(),
                    Some("https://accounts.x.ai/device?user_code=ABCD-EFGH")
                );
                assert_eq!(code.as_deref(), Some("ABCD-EFGH"));
            }
            _ => panic!("expected SigningIn"),
        }

        // External command flow maps to SigningIn with the URL and no code
        let st = AuthState::Authenticating {
            request_seq: 2,
            handle: None,
            auth_url: Some("https://provider.example/login".into()),
            mode: AuthMode::Command,
        };
        match minimal_auth_hint(&st, &trust_done, true, false) {
            MinimalAuthHint::SigningIn { url, code } => {
                assert_eq!(url.as_deref(), Some("https://provider.example/login"));
                assert!(code.is_none());
            }
            _ => panic!("expected SigningIn"),
        }

        assert!(matches!(
            minimal_auth_hint(&AuthState::Done, &trust_done, true, false),
            MinimalAuthHint::Starting
        ));
        assert!(matches!(
            minimal_auth_hint(
                &AuthState::Pending {
                    error: Some("nope".into())
                },
                &trust_done,
                true,
                false
            ),
            MinimalAuthHint::Failed(_)
        ));
    }

    #[test]
    fn auth_hint_maps_pending_trust_after_auth() {
        let trust = TrustState::Pending {
            workspace: PathBuf::from("/tmp/untrusted-repo"),
        };
        match minimal_auth_hint(&AuthState::Done, &trust, true, false) {
            MinimalAuthHint::TrustFolder { workspace } => {
                assert_eq!(workspace, PathBuf::from("/tmp/untrusted-repo"));
            }
            _ => panic!("expected TrustFolder"),
        }

        // Access / ZDR gates suppress the trust question (matches welcome and the input interceptor)
        assert!(matches!(
            minimal_auth_hint(&AuthState::Done, &trust, false, false),
            MinimalAuthHint::Starting
        ));
        assert!(matches!(
            minimal_auth_hint(&AuthState::Done, &trust, true, true),
            MinimalAuthHint::Starting
        ));

        // Trust is not offered while auth is still in flight.
        assert!(matches!(
            minimal_auth_hint(&AuthState::Pending { error: None }, &trust, true, false),
            MinimalAuthHint::Connecting
        ));
    }

    #[test]
    fn sign_in_start_says_connecting_until_the_flow_is_known() {
        use xai_grok_pager::app::app_view::AuthMode;

        let trust = TrustState::Done;
        assert!(matches!(
            minimal_auth_hint(&AuthState::Pending { error: None }, &trust, true, false),
            MinimalAuthHint::Connecting
        ));
        let unknown = AuthState::Authenticating {
            request_seq: 1,
            handle: None,
            auth_url: None,
            mode: AuthMode::Pending,
        };
        assert!(matches!(
            minimal_auth_hint(&unknown, &trust, true, false),
            MinimalAuthHint::Connecting
        ));
        let browser = AuthState::Authenticating {
            request_seq: 2,
            handle: None,
            auth_url: None,
            mode: AuthMode::Command,
        };
        assert!(matches!(
            minimal_auth_hint(&browser, &trust, true, false),
            MinimalAuthHint::SigningIn {
                url: None,
                code: None
            }
        ));

        let theme = Theme::current();
        let area = Rect::new(0, 0, 40, 4);
        let mut buf = Buffer::empty(area);
        render_auth(&mut buf, area, &theme, &MinimalAuthHint::Connecting);
        let text = crate::buffer_text(&buf);
        assert!(text.contains("Connecting\u{2026}"), "{text:?}");
        assert!(!text.contains("Opening your browser"), "{text:?}");
        assert!(!text.contains("Sign in to Grok"), "{text:?}");
        assert_eq!(auth_hint_rows(&MinimalAuthHint::Connecting, 40), 1);

        let mut browser_buf = Buffer::empty(area);
        render_auth(
            &mut browser_buf,
            area,
            &theme,
            &MinimalAuthHint::SigningIn {
                url: None,
                code: None,
            },
        );
        let browser_text = crate::buffer_text(&browser_buf);
        assert!(
            browser_text.contains("Opening your browser"),
            "{browser_text:?}"
        );
    }

    #[test]
    fn render_auth_shows_url_and_code() {
        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 12);
        let mut buf = Buffer::empty(area);
        let hint = MinimalAuthHint::SigningIn {
            url: Some("https://accounts.x.ai/device?user_code=ABCD-EFGH".into()),
            code: Some("ABCD-EFGH".into()),
        };
        render_auth(&mut buf, area, &theme, &hint);
        let text = crate::buffer_text(&buf);
        assert!(text.contains("Sign in to Grok"), "header: {text:?}");
        assert!(text.contains("accounts.x.ai/device"), "url: {text:?}");
        assert!(text.contains("ABCD-EFGH"), "device code: {text:?}");
        assert!(
            text.contains("Make sure your browser shows this code."),
            "caption: {text:?}"
        );
        assert!(!text.contains("Code:"), "the code is not prefixed: {text:?}");
        assert!(
            text.contains("Waiting for approval"),
            "waiting line: {text:?}"
        );
    }

    #[test]
    fn device_sign_in_shows_the_code_then_its_caption() {
        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 12);
        let mut buf = Buffer::empty(area);
        let hint = MinimalAuthHint::SigningIn {
            url: Some("https://accounts.x.ai/device?user_code=ABCD-EFGH".into()),
            code: Some("ABCD-EFGH".into()),
        };
        render_auth(&mut buf, area, &theme, &hint);
        let rows: Vec<String> = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buf.cell((x, y)).map(|cell| cell.symbol()).unwrap_or_default())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect();
        let code_row = rows
            .iter()
            .position(|row| row == "ABCD-EFGH")
            .expect("the code is a row by itself");
        let caption = rows
            .iter()
            .skip(code_row + 1)
            .find(|row| !row.is_empty())
            .expect("a caption follows the code");
        assert_eq!(caption, "Make sure your browser shows this code.");
        let code_cell = buf.cell((0, code_row as u16)).expect("code cell");
        assert_eq!(code_cell.fg, theme.text_primary);
        assert!(code_cell.modifier.contains(ratatui::style::Modifier::BOLD));
        let narrow = auth_hint_rows(&hint, 16);
        let wide = auth_hint_rows(&hint, 80);
        assert!(
            narrow > wide,
            "a narrow caption takes another row: narrow {narrow}, wide {wide}"
        );
    }

    #[test]
    fn render_auth_shows_trust_question() {
        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 14);
        let mut buf = Buffer::empty(area);
        let hint = MinimalAuthHint::TrustFolder {
            workspace: PathBuf::from("/home/agent/project"),
        };
        render_auth(&mut buf, area, &theme, &hint);
        let text = crate::buffer_text(&buf);
        assert!(
            text.contains("Do you trust the contents of this directory?"),
            "question: {text:?}"
        );
        assert!(
            text.contains("/home/agent/project"),
            "workspace path: {text:?}"
        );
        assert!(text.contains("y Yes, proceed"), "yes key: {text:?}");
        assert!(text.contains("n No, quit"), "no key: {text:?}");
        assert!(!text.contains("y  Yes"), "one space after y: {text:?}");
        assert!(!text.contains("n  No"), "one space after n: {text:?}");
        assert!(text.contains("Enter or y to trust"), "hint line: {text:?}");
        assert!(text.contains("posing security risks"), "warning: {text:?}");
    }

    #[test]
    fn failed_sign_in_wraps_on_a_narrow_row() {
        let err = "the token endpoint refused this refresh token and asked for a new login";
        let hint = MinimalAuthHint::Failed(err.to_string());
        assert!(auth_hint_rows(&hint, 16) > auth_hint_rows(&hint, 80));
        let theme = Theme::current();
        let area = Rect::new(0, 0, 16, 16);
        let mut buf = Buffer::empty(area);
        render_auth(&mut buf, area, &theme, &hint);
        let text = crate::buffer_text(&buf);
        assert!(
            !text.contains("Sign-in failed"),
            "the heading is not painted: {text:?}"
        );
        assert!(text.contains("endpoint"), "{text:?}");
        assert!(text.contains("login"), "{text:?}");
        let message = buf.cell((0, 0)).expect("error cell");
        assert_eq!(message.symbol(), "t");
        assert_eq!(
            message.fg, theme.accent_error,
            "the error message must be red"
        );
    }

    #[test]
    fn failed_sign_in_shows_only_the_error_in_red() {
        let err = "the token endpoint refused this refresh token";
        let hint = MinimalAuthHint::Failed(err.to_string());
        let theme = Theme::current();
        let area = Rect::new(0, 0, 80, 8);
        let mut buf = Buffer::empty(area);
        render_auth(&mut buf, area, &theme, &hint);
        let text = crate::buffer_text(&buf);
        assert!(text.contains(err), "{text:?}");
        assert!(!text.contains("Sign-in failed"), "{text:?}");
        let rows = auth_hint_rows(&hint, 80);
        assert_eq!(rows, 1, "one line, no heading or blank row");
        for x in 0..err.len() as u16 {
            let cell = buf.cell((x, 0)).expect("message cell");
            assert_eq!(cell.fg, theme.accent_error, "column {x}");
        }
    }

    #[test]
    fn auth_hint_rows_covers_trust_path_wrap() {
        let long = "x".repeat(200);
        let hint = MinimalAuthHint::TrustFolder {
            workspace: PathBuf::from(long),
        };
        let rows = auth_hint_rows(&hint, 40);
        // The 200-char path wraps to 5 rows at width 40, so the total sits well above the fixed rows
        assert!(rows >= 12, "expected room for wrapped path, got {rows}");
    }
}
