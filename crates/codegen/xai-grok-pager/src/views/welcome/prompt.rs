use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::render::SafeBuf;
use crate::theme::Theme;
use crate::views::prompt_widget::{PromptInfo, PromptStyle, PromptWidget};

use super::WelcomePromptFocus;

pub fn prompt_inset(compact: bool) -> u16 {
    if compact { 0 } else { 2 }
}

fn prompt_area_inset(compact: bool) -> u16 {
    if compact {
        super::PROMPT_GUTTER
    } else {
        prompt_inset(false)
    }
}

const CHROME_PAD: u16 = 2;

fn prompt_style(focus: WelcomePromptFocus, compact: bool) -> PromptStyle {
    PromptStyle {
        focused: focus == WelcomePromptFocus::Focused,
        show_prefix: true,
        vpad_top: 1,
        compact,
        chrome: true,
        chrome_pad_left: CHROME_PAD,
        chrome_pad_right: CHROME_PAD,
        placeholder_override: Some("Type a message..."),
        ..PromptStyle::default()
    }
}

/// Measured with the style [`render_prompt`] draws with, so the layout reserves the rows the draft will paint into.
pub fn desired_prompt_height(
    prompt: &PromptWidget,
    content_width: u16,
    compact: bool,
    max_height: u16,
) -> u16 {
    let inset = prompt_area_inset(compact);
    // Focus only tints, never changes the row count
    let style = prompt_style(WelcomePromptFocus::Focused, compact);
    prompt.desired_height(
        content_width.saturating_sub(inset * 2),
        &style,
        true,
        max_height,
    )
}

/// Returns the cursor position and the post-flush output that carries terminal-overlay ownership.
pub fn render_prompt(
    area: Rect,
    buf: &mut Buffer,
    focus: WelcomePromptFocus,
    prompt: &mut PromptWidget,
    info: &PromptInfo<'_>,
    compact: bool,
) -> (
    Option<(u16, u16)>,
    Option<crate::terminal::overlay::PostFlush>,
) {
    let style = prompt_style(focus, compact);

    // A two-column inset on a row narrower than eight leaves the composer below its draw floor.
    let mut inset = prompt_area_inset(compact);
    if area.height > 0 && area.width.saturating_sub(inset.saturating_mul(2)) < 4 {
        inset = 0;
    }
    let inset_area = Rect {
        x: area.x.saturating_add(inset),
        y: area.y,
        width: area.width.saturating_sub(inset.saturating_mul(2)),
        height: area.height,
    };

    // The shared composer draws nothing below four columns. Welcome still paints a sliver.
    if inset_area.height > 0 && inset_area.width < 4 {
        paint_prompt_sliver(inset_area, buf, prompt);
        return (None, None);
    }

    let result = prompt.draw(buf, inset_area, None, &style, Some(info), None);

    (result.cursor_pos, result.post_flush_escapes.map(Into::into))
}

/// One row of the welcome prompt when the composer would refuse the area.
/// The arrow fits in two columns; a one-column row keeps its first character.
pub(super) fn paint_prompt_sliver(area: Rect, buf: &mut Buffer, prompt: &PromptWidget) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let theme = Theme::current();
    let style = Style::default().fg(theme.text_primary);
    let arrow = crate::glyphs::prompt_arrow();
    let arrow_w = crate::glyphs::PROMPT_ARROW_WIDTH;
    if area.width >= arrow_w {
        buf.set_span_safe(area.x, area.y, &Span::styled(arrow, style), arrow_w);
        let room = area.width.saturating_sub(arrow_w);
        if room == 0 {
            return;
        }
        let x = area.x.saturating_add(arrow_w);
        let draft = prompt.text();
        let body = if draft.trim().is_empty() {
            "Type a message..."
        } else {
            draft
        };
        let body = body.lines().next().unwrap_or("");
        let clipped = crate::render::line_utils::truncate_str(body, room as usize);
        if clipped.is_empty() {
            return;
        }
        let drawn = unicode_width::UnicodeWidthStr::width(clipped.as_str()) as u16;
        buf.set_span_safe(x, area.y, &Span::styled(clipped, style), drawn.min(room));
    } else {
        let first = arrow.chars().next().unwrap_or('>');
        buf.set_span_safe(area.x, area.y, &Span::styled(first.to_string(), style), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::image::{GraphicsProtocol, set_protocol_for_test};
    use crossterm::Command;

    fn png() -> [u8; 8] {
        [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']
    }

    #[test]
    fn prompt_post_flush_keeps_ownership_when_plain_bytes_are_appended() {
        let _guard = set_protocol_for_test(GraphicsProtocol::Kitty);
        crate::terminal::overlay::reset_owner();
        let _ = crate::terminal::overlay::static_image(&png(), 20, 10, 0, 0, 71)
            .unwrap()
            .commit();
        let area = Rect::new(0, 0, 80, 3);
        let mut buf = Buffer::empty(area);
        let mut prompt = PromptWidget::new();
        let info = PromptInfo {
            model_name: "test",
            flags: &[],
            multiline: false,
            usage_warning: None,
            usage_warning_critical: false,
        };

        let (_, post_flush) = render_prompt(
            area,
            &mut buf,
            WelcomePromptFocus::Focused,
            &mut prompt,
            &info,
            false,
        );
        let mut post_flush = post_flush.expect("welcome clear");
        let mut cursor_bytes = String::new();
        let _ = crate::terminal::SetPointerCursor.write_ansi(&mut cursor_bytes);
        assert!(!cursor_bytes.is_empty());
        post_flush.append_plain(&cursor_bytes);
        assert!(post_flush.as_str().contains("a=d"));
        assert!(post_flush.as_str().ends_with(cursor_bytes.as_str()));
        assert!(
            !crate::terminal::overlay::static_image(&png(), 20, 10, 0, 0, 71)
                .unwrap()
                .as_str()
                .contains("a=T"),
            "constructing welcome output must not commit its clear"
        );

        let mut emitted = Vec::new();
        post_flush.write_to(&mut emitted).unwrap();
        assert!(
            crate::terminal::overlay::static_image(&png(), 20, 10, 0, 0, 71)
                .unwrap()
                .as_str()
                .contains("a=T"),
            "writing welcome output must commit its clear"
        );
    }

    #[test]
    fn desired_prompt_height_grows_per_draft_line_up_to_max() {
        let mut prompt = PromptWidget::new();
        assert_eq!(
            desired_prompt_height(&prompt, 80, false, 20),
            super::super::PROMPT_HEIGHT
        );

        prompt.set_text("one\ntwo\nthree");
        assert_eq!(
            desired_prompt_height(&prompt, 80, false, 20),
            super::super::PROMPT_HEIGHT + 2
        );

        prompt.set_text(&["line"; 30].join("\n"));
        assert_eq!(desired_prompt_height(&prompt, 80, false, 20), 20);
    }

    #[test]
    fn desired_prompt_height_counts_wrapped_rows_at_the_drawn_width() {
        let mut prompt = PromptWidget::new();
        prompt.set_text(&"word ".repeat(40));
        let narrow = desired_prompt_height(&prompt, 40, false, 40);
        let wide = desired_prompt_height(&prompt, 400, false, 40);
        assert_eq!(wide, super::super::PROMPT_HEIGHT);
        assert!(narrow > wide, "narrow={narrow} wide={wide}");
    }

    /// Compact mode insets the drawn box by `PROMPT_GUTTER` instead of 2, so the measured wrap width must follow the draw width.
    #[test]
    fn desired_prompt_height_measures_at_the_compact_draw_width() {
        let mut prompt = PromptWidget::new();
        prompt.set_text(&"word ".repeat(40));
        for width in [40u16, 60, 80] {
            let regular = desired_prompt_height(&prompt, width, false, 40);
            let compact = desired_prompt_height(&prompt, width, true, 40);
            let regular_text_width = width.saturating_sub(prompt_area_inset(false) * 2);
            let compact_text_width = width.saturating_sub(prompt_area_inset(true) * 2);
            assert!(
                compact <= regular,
                "width {width}: compact {compact} regular {regular}"
            );
            assert_eq!(
                compact,
                prompt.desired_height(
                    compact_text_width,
                    &prompt_style(WelcomePromptFocus::Focused, true),
                    true,
                    40
                ),
                "width {width}"
            );
            assert_ne!(regular_text_width, compact_text_width);
        }
    }
}
