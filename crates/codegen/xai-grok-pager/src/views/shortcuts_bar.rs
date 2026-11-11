//! The bar accepts `&[HintItem]` from any source (action registry, prompt widget, scrollback state); each view builds its own hints dynamically.
//!
//! When a `PendingAction` is active (double-press confirmation), the bar replaces all hints with "press again to {label}".

use std::borrow::Cow;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::input::key::KeyShortcut;
use crate::theme::Theme;

/// A single hint for the shortcuts bar. It carries the keys as structured data; the bar decides how
/// they render. Views build these dynamically from the registry, widget keymaps, or local state.
#[derive(Debug, Clone)]
pub struct HintItem {
    /// Keys to display. Multiple keys are shown joined with "/" (e.g., j/k).
    pub keys: Vec<KeyShortcut>,
    /// Short label for the bottom bar (e.g., "send", "nav", "cancel").
    pub label: Cow<'static, str>,
    /// Optional custom key text, used when the bar paints a single key (and in the cheatsheet).
    /// With more than one key (`keys.len() > 1`) the bar ignores this and builds the text from the structured keys.
    pub custom_display: Option<&'static str>,
    /// Longer description for the all-shortcuts cheatsheet (e.g., "Send prompt to agent").
    /// When `None`, falls back to `label`.
    pub description: Option<Cow<'static, str>>,
    /// When true, the hint survives compact-mode truncation and renders regardless of `max_visible`.
    /// Use for hints that should be discoverable in every scrollback context (e.g. nav, turn, mode).
    pub pinned: bool,
}

impl HintItem {
    /// Single-key hint.
    pub fn new(key: KeyShortcut, label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            keys: vec![key],
            label: label.into(),
            custom_display: None,
            description: None,
            pinned: false,
        }
    }

    /// Paired-key hint (e.g., j/k for nav, h/l for turn).
    pub fn paired(a: KeyShortcut, b: KeyShortcut, label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            keys: vec![a, b],
            label: label.into(),
            custom_display: None,
            description: None,
            pinned: false,
        }
    }

    /// Mark this hint as pinned: it is always shown in the compact shortcuts bar, even when the hint list exceeds `max_visible`.
    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    fn bar_key_display(&self) -> String {
        bar_key_segments(self)
            .into_iter()
            .map(|seg| seg.text)
            .collect()
    }
}

struct BarKeySeg {
    text: String,
    is_join: bool,
}

fn bar_key_segments(hint: &HintItem) -> Vec<BarKeySeg> {
    if hint.keys.len() <= 1 {
        let text = if let Some(d) = hint.custom_display {
            d.to_string()
        } else if let Some(k) = hint.keys.first() {
            k.display()
        } else {
            String::new()
        };
        if text.is_empty() {
            return Vec::new();
        }
        return vec![BarKeySeg {
            text,
            is_join: false,
        }];
    }

    let Some(first) = hint.keys.first() else {
        return Vec::new();
    };
    let shared = hint.keys.iter().all(|k| k.modifiers == first.modifiers);
    let mut segs = Vec::new();
    if shared {
        let prefix = first.modifiers_prefix();
        if !prefix.is_empty() {
            segs.push(BarKeySeg {
                text: prefix,
                is_join: false,
            });
        }
        for (i, k) in hint.keys.iter().enumerate() {
            if i > 0 {
                segs.push(BarKeySeg {
                    text: "/".into(),
                    is_join: true,
                });
            }
            segs.push(BarKeySeg {
                text: k.code_display(),
                is_join: false,
            });
        }
    } else {
        for (i, k) in hint.keys.iter().enumerate() {
            if i > 0 {
                segs.push(BarKeySeg {
                    text: "/".into(),
                    is_join: true,
                });
            }
            segs.push(BarKeySeg {
                text: k.display(),
                is_join: false,
            });
        }
    }
    segs
}

pub struct ShortcutsBar<'a> {
    hints: &'a [HintItem],
    /// If set, replaces all hints with "press again to {label}".
    pending_confirmation: Option<PendingHint>,
    /// Right-aligned text (e.g. team name).
    right_text: Option<&'a str>,
    /// Compact mode: render only the first `max_visible` hints from `hints`, then always append `help_hint` (e.g. the "all shortcuts" modal trigger).
    /// When None, all hints are rendered.
    compact: Option<CompactConfig>,
}

pub struct CompactConfig {
    /// Maximum number of items to render from the hint list before the trailing help hint.
    pub max_visible: usize,
    /// The trailing help hint (typically the binding for the all-shortcuts modal).
    /// It is always rendered when set, even if the hint list is empty.
    pub help_hint: Option<HintItem>,
}

/// Info needed to render the "press again" hint.
#[derive(Clone, Copy)]
pub struct PendingHint {
    pub shortcut: KeyShortcut,
    pub label: &'static str,
}

impl<'a> ShortcutsBar<'a> {
    pub fn new(hints: &'a [HintItem]) -> Self {
        Self {
            hints,
            pending_confirmation: None,
            right_text: None,
            compact: None,
        }
    }

    /// Render only the first `max_visible` hints, then append `help_hint` (typically the binding that opens the all-shortcuts modal).
    pub fn compact(mut self, max_visible: usize, help_hint: Option<HintItem>) -> Self {
        self.compact = Some(CompactConfig {
            max_visible,
            help_hint,
        });
        self
    }

    /// Set the pending confirmation hint (replaces all normal hints).
    pub fn with_pending(mut self, pending: Option<PendingHint>) -> Self {
        self.pending_confirmation = pending;
        self
    }

    /// Set right-aligned text (e.g. team name).
    pub fn with_right_text(mut self, text: Option<&'a str>) -> Self {
        self.right_text = text;
        self
    }
}

impl Widget for ShortcutsBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let theme = Theme::current();

        let bg_style = Style::default()
            .bg(theme.bg_base)
            .fg(theme.gray)
            .remove_modifier(Modifier::all());
        // Clear both content and style: set_style only patches style and would leave old text from previous renders
        for x in area.x..area.right() {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.reset();
                cell.set_style(bg_style);
            }
        }

        let key_style = Style::default()
            .fg(theme.text_secondary)
            .bg(theme.bg_base)
            .add_modifier(Modifier::BOLD);

        // muted(): DIM on the terminal theme, where `gray` is Reset and the
        // label would otherwise be indistinguishable from the bold key.
        let action_style = theme
            .muted()
            .bg(theme.bg_base)
            .remove_modifier(Modifier::BOLD);

        // If pending confirmation, show only "press again to {label}"
        if let Some(pending) = &self.pending_confirmation {
            let key_text = pending.shortcut.display();
            let label = format!("press again to {}", pending.label);
            paint_fitted_hint(
                buf,
                area,
                area.x,
                &key_text,
                &label,
                key_style,
                action_style,
            );
            return;
        }

        let sep_style = Style::default()
            .fg(theme.gray)
            .bg(theme.bg_base)
            .add_modifier(Modifier::DIM)
            .remove_modifier(Modifier::BOLD);

        let mut x = area.x;

        let effective = compute_effective_hints(self.hints, self.compact.as_ref());

        for (i, hint) in effective.iter().enumerate() {
            let sep_width = if i > 0 { HINT_GAP_WIDTH } else { 0 };
            let room = area.right().saturating_sub(x);
            if room <= sep_width {
                break;
            }
            let body_room = room - sep_width;
            let key_text = hint.bar_key_display();
            let key_width = key_text.width() as u16;
            let label_width = hint.label.width() as u16;
            let full = key_width.saturating_add(1).saturating_add(label_width);
            if full > body_room && key_width.saturating_add(2) > body_room {
                // Not enough for the key, the colon, and an ellipsis. Drop the hint whole.
                break;
            }
            if sep_width > 0 {
                let sep = Span::styled("  │  ", sep_style);
                buf.set_span(x, area.y, &sep, sep_width);
                x = x.saturating_add(sep_width);
            }
            if full <= body_room {
                x = paint_hint_keys(buf, x, area.y, hint, key_style, action_style);
                let colon = Span::styled(":", action_style);
                buf.set_span(x, area.y, &colon, 1);
                x = x.saturating_add(1);
                let action_span = Span::styled(hint.label.as_ref(), action_style);
                buf.set_span(x, area.y, &action_span, label_width);
                x = x.saturating_add(label_width);
            } else {
                x = paint_fitted_hint(
                    buf,
                    area,
                    x,
                    &key_text,
                    hint.label.as_ref(),
                    key_style,
                    action_style,
                );
                break;
            }
        }

        // Right-aligned text (team name etc.). Shorten it into the gap that is left; never paint over a hint.
        if let Some(text) = self.right_text {
            let right_style = theme.muted().bg(theme.bg_base);
            let gap_start = x.saturating_add(2);
            if gap_start < area.right() {
                let gap = area.right().saturating_sub(gap_start) as usize;
                let display = format!("{text} ");
                let shown = crate::util::truncate_to_width(&display, gap);
                let rw = shown.width() as u16;
                if rw > 0 {
                    let rx = area.right().saturating_sub(rw);
                    if rx >= gap_start {
                        let right_span = Span::styled(shown.as_ref(), right_style);
                        buf.set_span(rx, area.y, &right_span, rw);
                    }
                }
            }
        }
    }
}

const HINT_GAP_WIDTH: u16 = 5;

/// Paint `key:label`, shortening the label (then the key) so the write stays inside `area`.
fn paint_fitted_hint(
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    key_text: &str,
    label: &str,
    key_style: Style,
    action_style: Style,
) -> u16 {
    let room = area.right().saturating_sub(x) as usize;
    if room == 0 {
        return x;
    }
    let key_width = key_text.width();
    let label_width = label.width();
    if key_width.saturating_add(1).saturating_add(label_width) <= room {
        let mut cursor = x;
        cursor = paint_key_run(buf, cursor, area.y, key_text, key_style);
        let colon = Span::styled(":", action_style);
        buf.set_span(cursor, area.y, &colon, 1);
        cursor = cursor.saturating_add(1);
        let action_span = Span::styled(label, action_style);
        buf.set_span(cursor, area.y, &action_span, label_width as u16);
        return cursor.saturating_add(label_width as u16);
    }
    if key_width.saturating_add(2) <= room {
        let mut cursor = x;
        cursor = paint_key_run(buf, cursor, area.y, key_text, key_style);
        let colon = Span::styled(":", action_style);
        buf.set_span(cursor, area.y, &colon, 1);
        cursor = cursor.saturating_add(1);
        let budget = area.right().saturating_sub(cursor) as usize;
        let short = crate::util::truncate_to_width(label, budget);
        let short_width = short.width() as u16;
        let action_span = Span::styled(short.as_ref(), action_style);
        buf.set_span(cursor, area.y, &action_span, short_width);
        return cursor.saturating_add(short_width);
    }
    let short = crate::util::truncate_to_width(key_text, room);
    let short_width = short.width() as u16;
    let key_span = Span::styled(short.as_ref(), key_style);
    buf.set_span(x, area.y, &key_span, short_width);
    x.saturating_add(short_width)
}

fn paint_key_run(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) -> u16 {
    if text.is_empty() {
        return x;
    }
    let w = text.width() as u16;
    let span = Span::styled(text, style);
    buf.set_span(x, y, &span, w);
    x + w
}

fn paint_hint_keys(
    buf: &mut Buffer,
    mut x: u16,
    y: u16,
    hint: &HintItem,
    key_style: Style,
    sep_style: Style,
) -> u16 {
    for seg in bar_key_segments(hint) {
        let style = if seg.is_join { sep_style } else { key_style };
        x = paint_key_run(buf, x, y, &seg.text, style);
    }
    x
}

/// Without `compact`: returns every hint from the input slice. With `compact`: pinned hints are
/// always included; the remaining `max_visible − pinned_count` slots take unpinned hints in their
/// original order.
pub fn compute_effective_hints<'a>(
    hints: &'a [HintItem],
    compact: Option<&'a CompactConfig>,
) -> Vec<&'a HintItem> {
    if let Some(cfg) = compact {
        let pinned_count = hints.iter().filter(|h| h.pinned).count();
        let unpinned_budget = cfg.max_visible.saturating_sub(pinned_count);
        let mut unpinned_used = 0;
        let mut v: Vec<&HintItem> = hints
            .iter()
            .filter(|h| {
                if h.pinned {
                    true
                } else if unpinned_used < unpinned_budget {
                    unpinned_used += 1;
                    true
                } else {
                    false
                }
            })
            .collect();
        if let Some(ref h) = cfg.help_hint {
            v.push(h);
        }
        v
    } else {
        hints.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key;

    fn h(label: &'static str, k: crate::input::key::KeyShortcut) -> HintItem {
        HintItem::new(k, label)
    }

    #[test]
    fn full_mode_returns_all_hints() {
        let hints = vec![h("a", key!('a')), h("b", key!('b')), h("c", key!('c'))];
        let out = compute_effective_hints(&hints, None);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn compact_takes_first_n_then_appends_help() {
        let hints = vec![h("a", key!('a')), h("b", key!('b')), h("c", key!('c'))];
        let help = h("shortcuts", key!('/', CONTROL));
        let cfg = CompactConfig {
            max_visible: 2,
            help_hint: Some(help),
        };
        let out = compute_effective_hints(&hints, Some(&cfg));
        assert_eq!(out.len(), 3); // two visible hints plus the help hint
        let [a, b, help_row] = out.as_slice() else {
            panic!("expected two visible hints plus help: {out:?}");
        };
        assert_eq!(a.label, "a");
        assert_eq!(b.label, "b");
        assert_eq!(help_row.label, "shortcuts");
    }

    #[test]
    fn compact_help_hint_renders_even_with_empty_hint_list() {
        let hints: Vec<HintItem> = vec![];
        let help = h("shortcuts", key!('/', CONTROL));
        let cfg = CompactConfig {
            max_visible: 2,
            help_hint: Some(help),
        };
        let out = compute_effective_hints(&hints, Some(&cfg));
        assert_eq!(out.len(), 1);
        assert_eq!(out.first().map(|h| h.label.as_ref()), Some("shortcuts"));
    }

    #[test]
    fn compact_without_help_just_truncates() {
        let hints = vec![h("a", key!('a')), h("b", key!('b')), h("c", key!('c'))];
        let cfg = CompactConfig {
            max_visible: 2,
            help_hint: None,
        };
        let out = compute_effective_hints(&hints, Some(&cfg));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn compact_max_visible_larger_than_input_is_safe() {
        let hints = vec![h("a", key!('a'))];
        let cfg = CompactConfig {
            max_visible: 10,
            help_hint: None,
        };
        let out = compute_effective_hints(&hints, Some(&cfg));
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn compact_pinned_hints_always_included() {
        // a, b, c are unpinned; d, e are pinned
        // With max_visible 3 the two pinned hints leave one unpinned slot, which goes to a
        let hints = vec![
            h("a", key!('a')),
            h("b", key!('b')),
            h("c", key!('c')),
            h("d", key!('d')).pinned(),
            h("e", key!('e')).pinned(),
        ];
        let cfg = CompactConfig {
            max_visible: 3,
            help_hint: None,
        };
        let out = compute_effective_hints(&hints, Some(&cfg));
        let labels: Vec<&str> = out.iter().map(|h| h.label.as_ref()).collect();
        assert_eq!(labels, vec!["a", "d", "e"]);
    }

    #[test]
    fn compact_pinned_preserves_original_order() {
        // The pinned hint sits between unpinned ones and keeps its position
        let hints = vec![
            h("a", key!('a')),
            h("nav", key!('j')).pinned(),
            h("b", key!('b')),
            h("c", key!('c')),
        ];
        let cfg = CompactConfig {
            max_visible: 3,
            help_hint: None,
        };
        let out = compute_effective_hints(&hints, Some(&cfg));
        let labels: Vec<&str> = out.iter().map(|h| h.label.as_ref()).collect();
        // One pinned hint leaves a budget of two unpinned ones
        assert_eq!(labels, vec!["a", "nav", "b"]);
    }

    #[test]
    fn compact_all_pinned_exceeding_max_visible() {
        // More pinned hints than max_visible: every pinned hint still shows
        let hints = vec![
            h("a", key!('a')).pinned(),
            h("b", key!('b')).pinned(),
            h("c", key!('c')).pinned(),
            h("d", key!('d')),
        ];
        let cfg = CompactConfig {
            max_visible: 2,
            help_hint: None,
        };
        let out = compute_effective_hints(&hints, Some(&cfg));
        let labels: Vec<&str> = out.iter().map(|h| h.label.as_ref()).collect();
        // All three pinned hints show; the unpinned budget is zero
        assert_eq!(labels, vec!["a", "b", "c"]);
    }

    fn render_hints(hints: &[HintItem]) -> Buffer {
        use ratatui::layout::Rect;
        use ratatui::widgets::Widget;

        let area = Rect::new(0, 0, 48, 1);
        let mut buf = Buffer::empty(area);
        ShortcutsBar::new(hints).render(area, &mut buf);
        buf
    }

    fn leading_text(buf: &Buffer, n: usize) -> String {
        let mut text = String::new();
        for col in 0..n {
            text.push_str(buf.cell((col as u16, 0)).expect("cell").symbol());
        }
        text
    }

    /// Terminal theme: the action label renders via `muted()` — the DIM attribute — so it reads dimmer
    /// than the bold key even though both sit on the terminal's default fg (`gray` and `text_secondary`
    /// are both muted slots there). RGB themes keep the plain gray label fg.
    #[test]
    fn label_is_dimmer_than_key_on_terminal_theme() {
        use ratatui::style::Modifier;

        let _guard = crate::theme::cache::pin_theme();
        let hints = [h("send", key!('x', CONTROL))];

        crate::theme::cache::set(crate::theme::ThemeKind::Terminal);
        let buf = render_hints(&hints);
        // "Ctrl+x:send" — key cell at col 0, label cell after the colon.
        let text = leading_text(&buf, 12);
        let label_x = text.find(":send").expect("label rendered") as u16 + 1;
        let key = buf.cell((0, 0)).unwrap().style();
        let label = buf.cell((label_x, 0)).unwrap().style();
        assert!(key.add_modifier.contains(Modifier::BOLD));
        assert!(
            !key.add_modifier.contains(Modifier::DIM),
            "key must stay full-brightness, got {key:?}"
        );
        assert!(
            label.add_modifier.contains(Modifier::DIM),
            "terminal-theme label must be dimmed, got {label:?}"
        );

        crate::theme::cache::set(crate::theme::ThemeKind::GrokNight);
        let buf = render_hints(&hints);
        let label = buf.cell((label_x, 0)).unwrap().style();
        assert_eq!(label.fg, Some(Theme::current().gray), "RGB keeps gray");
        assert!(!label.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn multi_key_shared_mod_compact_join_slash_uses_label_style() {
        use ratatui::style::Modifier;

        // Pinned: asserts exact ambient-theme colors.
        let _guard = crate::theme::cache::pin_theme();
        let theme = Theme::current();
        let key_fg = theme.text_secondary;
        let action_fg = theme.gray;

        let hints = [HintItem::paired(
            key!('[', CONTROL),
            key!(']', CONTROL),
            "prev/next agent",
        )];
        let buf = render_hints(&hints);

        assert!(
            leading_text(&buf, 9).starts_with("Ctrl+[/]:"),
            "expected compact Ctrl+[/]:, got {:?}",
            leading_text(&buf, 12)
        );

        // In "Ctrl+[/]:prev/next agent" the join slash lands at col 6
        let slash = buf.cell((6, 0)).expect("slash cell");
        assert_eq!(slash.symbol(), "/");
        assert_eq!(
            slash.style().fg,
            Some(action_fg),
            "join slash must match label gray, got {:?}",
            slash.style().fg
        );
        assert!(
            !slash.style().add_modifier.contains(Modifier::BOLD),
            "join slash must not be bold key style"
        );

        let open = buf.cell((5, 0)).expect("open bracket");
        assert_eq!(open.symbol(), "[");
        assert_eq!(open.style().fg, Some(key_fg));
        assert!(open.style().add_modifier.contains(Modifier::BOLD));

        let close = buf.cell((7, 0)).expect("close bracket");
        assert_eq!(close.symbol(), "]");
        assert_eq!(close.style().fg, Some(key_fg));
        assert!(close.style().add_modifier.contains(Modifier::BOLD));

        // "Ctrl+[/]:" is 9 cols and "prev" is 4, putting the label "/" at col 13
        let label_slash = buf.cell((13, 0)).expect("label slash cell");
        assert_eq!(label_slash.symbol(), "/");
        assert_eq!(label_slash.style().fg, Some(action_fg));
    }

    #[test]
    fn multi_key_ignores_custom_display() {
        use ratatui::style::Modifier;

        // Pinned: asserts exact ambient-theme colors.
        let _guard = crate::theme::cache::pin_theme();
        let theme = Theme::current();
        let action_fg = theme.gray;

        let mut item = HintItem::paired(key!('[', CONTROL), key!(']', CONTROL), "cycle");
        item.custom_display = Some("WRONG");
        let buf = render_hints(&[item]);

        let text = leading_text(&buf, 12);
        assert!(
            text.starts_with("Ctrl+[/]:"),
            "multi-key must ignore custom_display, got {text:?}"
        );
        assert!(
            !text.contains("WRONG"),
            "custom_display must not appear for multi-key, got {text:?}"
        );

        let join = buf.cell((6, 0)).expect("join");
        assert_eq!(join.symbol(), "/");
        assert_eq!(join.style().fg, Some(action_fg));
        assert!(!join.style().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn custom_display_with_slash_is_fully_key_styled() {
        use ratatui::style::Modifier;

        // Pinned: asserts exact ambient-theme colors.
        let _guard = crate::theme::cache::pin_theme();
        let theme = Theme::current();
        let key_fg = theme.text_secondary;

        let mut item = HintItem::new(key!(Null), "nav");
        item.custom_display = Some("\u{2191}/\u{2193}");
        let buf = render_hints(&[item]);

        // In "↑/↓:nav" all three key cells must be key-styled, including `/`
        for col in 0..3 {
            let cell = buf.cell((col, 0)).expect("key cell");
            assert_eq!(
                cell.style().fg,
                Some(key_fg),
                "col {col} ({}) must be key-styled",
                cell.symbol()
            );
            assert!(cell.style().add_modifier.contains(Modifier::BOLD));
        }
        assert_eq!(buf.cell((0, 0)).expect("up").symbol(), "\u{2191}");
        assert_eq!(buf.cell((1, 0)).expect("slash").symbol(), "/");
        assert_eq!(buf.cell((2, 0)).expect("down").symbol(), "\u{2193}");
        assert_eq!(buf.cell((3, 0)).expect("colon").symbol(), ":");
    }

    #[test]
    fn bare_slash_key_is_fully_key_styled() {
        use ratatui::style::Modifier;

        // Pinned: asserts exact ambient-theme colors.
        let _guard = crate::theme::cache::pin_theme();
        let theme = Theme::current();
        let key_fg = theme.text_secondary;

        let hints = [HintItem::new(key!('/'), "search")];
        let buf = render_hints(&hints);

        let slash = buf.cell((0, 0)).expect("slash cell");
        assert_eq!(slash.symbol(), "/");
        assert_eq!(slash.style().fg, Some(key_fg));
        assert!(
            slash.style().add_modifier.contains(Modifier::BOLD),
            "bare / must be key-styled, not a join separator"
        );
    }

    #[test]
    fn ctrl_slash_key_is_fully_key_styled() {
        use ratatui::style::Modifier;

        // Pinned: asserts exact ambient-theme colors.
        let _guard = crate::theme::cache::pin_theme();
        let theme = Theme::current();
        let key_fg = theme.text_secondary;

        let hints = [HintItem::new(key!('/', CONTROL), "search")];
        let buf = render_hints(&hints);

        // In "Ctrl+/" every cell is key-styled; the trailing / is not a join
        for col in 0..6 {
            let cell = buf.cell((col, 0)).expect("key cell");
            assert_eq!(
                cell.style().fg,
                Some(key_fg),
                "col {col} ({}) must be key-styled",
                cell.symbol()
            );
            assert!(cell.style().add_modifier.contains(Modifier::BOLD));
        }
        assert_eq!(buf.cell((5, 0)).expect("slash").symbol(), "/");
        assert_eq!(buf.cell((6, 0)).expect("colon").symbol(), ":");
    }

    #[test]
    fn multi_key_different_mods_uses_full_forms() {
        use ratatui::style::Modifier;

        // Pinned: asserts exact ambient-theme colors.
        let _guard = crate::theme::cache::pin_theme();
        let theme = Theme::current();
        let key_fg = theme.text_secondary;
        let action_fg = theme.gray;

        let hints = [HintItem::paired(key!(' ', CONTROL), key!(F(8)), "toggle")];
        let buf = render_hints(&hints);

        // "Ctrl+Space/F8:toggle"
        //  0123456789...
        // The join / lands at col 10
        let mut text = String::new();
        for col in 0..20 {
            text.push_str(buf.cell((col, 0)).expect("cell").symbol());
        }
        assert!(
            text.starts_with("Ctrl+Space/F8:"),
            "expected full forms, got {text:?}"
        );

        let join = buf.cell((10, 0)).expect("join slash");
        assert_eq!(join.symbol(), "/");
        assert_eq!(join.style().fg, Some(action_fg));
        assert!(!join.style().add_modifier.contains(Modifier::BOLD));

        let f = buf.cell((11, 0)).expect("F");
        assert_eq!(f.symbol(), "F");
        assert_eq!(f.style().fg, Some(key_fg));
        assert!(f.style().add_modifier.contains(Modifier::BOLD));
    }

    fn row_text(buf: &Buffer, width: u16) -> String {
        row_text_from(buf, 0, width)
    }

    fn row_text_from(buf: &Buffer, start: u16, end: u16) -> String {
        (start..end)
            .filter_map(|x| buf.cell((x, 0)).map(|c| c.symbol()))
            .collect()
    }

    #[test]
    fn narrow_footer_shortens_the_label_and_keeps_the_key() {
        let hints = [h("send", key!('x', CONTROL))];
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        ShortcutsBar::new(&hints).render(area, &mut buf);
        let text = row_text(&buf, area.width);
        assert!(
            text.starts_with("Ctrl+x:"),
            "the key stays whole: {text:?}"
        );
        assert!(text.contains('\u{2026}'), "the label is shortened: {text:?}");
        assert!(
            !text.contains("send"),
            "the full label must not run past the row: {text:?}"
        );
    }

    #[test]
    fn narrow_footer_drops_a_hint_that_does_not_fit() {
        let hints = [h("aa", key!('a')), h("bbbbbbbb", key!('b'))];
        let area = Rect::new(0, 0, 8, 1);
        let mut buf = Buffer::empty(area);
        ShortcutsBar::new(&hints).render(area, &mut buf);
        let text = row_text(&buf, area.width);
        assert!(text.contains("a:aa"), "the hint that fits stays: {text:?}");
        assert!(
            !text.contains('b'),
            "the next hint is dropped whole: {text:?}"
        );
        assert!(
            !text.contains('│'),
            "a dropped hint leaves no separator: {text:?}"
        );
    }

    #[test]
    fn narrow_footer_does_not_paint_the_team_name_over_a_hint() {
        let hints = [h("send", key!('x', CONTROL))];
        let area = Rect::new(0, 0, 16, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 24, 1));
        ShortcutsBar::new(&hints)
            .with_right_text(Some("TEAMNAME"))
            .render(area, &mut buf);
        let text = row_text(&buf, area.width);
        assert!(
            text.starts_with("Ctrl+x:send"),
            "the hint is not overwritten: {text:?}"
        );
        assert!(
            text.contains('\u{2026}'),
            "the team name shortens into the leftover gap: {text:?}"
        );
        let past = row_text_from(&buf, area.width, buf.area.width);
        assert!(
            past.trim().is_empty(),
            "nothing spills past the footer: {past:?}"
        );
    }
}
