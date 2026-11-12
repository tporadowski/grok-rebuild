//! Video playback overlay chrome (border, title, progress bar).
//!
//! The video frame itself is rendered via post-flush escape sequences by the caller, matching the image viewer pattern.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use unicode_width::UnicodeWidthStr;

use crate::prompt_images::VideoViewerState;
use crate::render::safe_buf::SafeBuf;
use crate::util::truncate_to_width;

/// Render the video viewer popup chrome.
/// Returns the popup `Rect`, or `None` if the area is too small.
pub fn render_video_overlay(
    buf: &mut Buffer,
    area: Rect,
    viewer: &VideoViewerState,
    bg: Color,
    text_fg: Color,
    border_fg: Color,
) -> Option<Rect> {
    if area.height < 8 || area.width < 20 {
        return None;
    }

    crate::render::color::dim_area(buf, area, bg, 0.5);

    // 90% centered popup.
    let popup_width = ((area.width as u32 * 90) / 100)
        .max(28)
        .min(area.width as u32) as u16;
    let popup_height = ((area.height as u32 * 90) / 100)
        .max(8)
        .min(area.height as u32) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_rect = Rect::new(popup_x, popup_y, popup_width, popup_height);

    ratatui::widgets::Clear.render(popup_rect, buf);
    buf.set_style(popup_rect, Style::default().fg(text_fg).bg(bg));

    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_fg).bg(bg))
        .style(Style::default().bg(bg))
        .render(popup_rect, buf);

    // Title centered in top border. Columns, not bytes: a CJK file name is
    // wider than its char count and must still leave both corners in place.
    let title = match viewer.title {
        Some(ref name) => format!(
            " {} ({}\u{00d7}{}) ",
            name, viewer.video_width, viewer.video_height
        ),
        None => format!(
            " Video ({}\u{00d7}{}) ",
            viewer.video_width, viewer.video_height
        ),
    };
    let title_style = Style::default()
        .fg(text_fg)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let max_title = popup_rect.width.saturating_sub(2) as usize;
    let title = truncate_to_width(&title, max_title).into_owned();
    let tw = title.width() as u16;
    let tx = popup_rect.x + (popup_rect.width.saturating_sub(tw)) / 2;
    buf.set_span_safe(tx, popup_rect.y, &Span::styled(&title, title_style), tw);

    // Progress bar on the bottom border row.
    render_progress_bar(buf, popup_rect, viewer, text_fg, border_fg, bg);

    Some(popup_rect)
}

/// Render the progress bar on the popup's bottom border row.
fn render_progress_bar(
    buf: &mut Buffer,
    popup_rect: Rect,
    viewer: &VideoViewerState,
    text_fg: Color,
    bar_dim: Color,
    bg: Color,
) {
    let bar_y = popup_rect.y + popup_rect.height.saturating_sub(1);
    let inner_width = popup_rect.width.saturating_sub(2) as usize;
    if inner_width == 0 {
        return;
    }

    let icon = if viewer.playing {
        "\u{25b6}"
    } else {
        "\u{23f8}"
    };
    let time_label = format!(
        "{icon} {}/{}  ",
        format_time(viewer.position_secs()),
        format_time(viewer.duration_secs),
    );
    let label_cols = time_label.width();
    // A narrow popup still shows the clock. The bar needs a few columns of its own.
    if label_cols + 4 >= inner_width {
        let shown = truncate_to_width(&time_label, inner_width).into_owned();
        let line = Line::from(Span::styled(
            shown,
            Style::default().fg(text_fg).bg(bg),
        ));
        buf.set_line_safe(popup_rect.x + 1, bar_y, &line, inner_width as u16);
        return;
    }
    let bar_width = inner_width - label_cols;

    let filled = ((viewer.progress() * bar_width as f64).round() as usize).min(bar_width);
    let empty = bar_width.saturating_sub(filled);

    let line = Line::from(vec![
        Span::styled(time_label, Style::default().fg(text_fg).bg(bg)),
        Span::styled(
            "\u{2501}".repeat(filled),
            Style::default().fg(text_fg).bg(bg),
        ),
        Span::styled(
            "\u{2500}".repeat(empty),
            Style::default().fg(bar_dim).bg(bg),
        ),
    ]);

    buf.set_line_safe(popup_rect.x + 1, bar_y, &line, inner_width as u16);
}

fn format_time(secs: f64) -> String {
    let total = secs.round() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_zero() {
        assert_eq!(format_time(0.0), "0:00");
    }

    #[test]
    fn format_time_short() {
        assert_eq!(format_time(5.4), "0:05");
    }

    #[test]
    fn format_time_minutes() {
        assert_eq!(format_time(90.0), "1:30");
    }

    fn paint(area: Rect, viewer: &VideoViewerState) -> (Option<Rect>, Buffer) {
        let mut buf = Buffer::empty(area);
        let popup = render_video_overlay(
            &mut buf,
            area,
            viewer,
            Color::Black,
            Color::White,
            Color::Gray,
        );
        (popup, buf)
    }

    fn row(buf: &Buffer, rect: Rect, y: u16) -> String {
        (rect.x..rect.x + rect.width)
            .filter_map(|x| buf.cell((x, y)).map(|cell| cell.symbol().to_owned()))
            .collect()
    }

    #[test]
    fn cjk_title_stays_inside_the_popup_corners() {
        let mut viewer = VideoViewerState::test_stub();
        viewer.title = Some("画面キャプチャの長いファイル名.mp4".into());
        viewer.video_width = 1920;
        viewer.video_height = 1080;
        viewer.duration_secs = 65.0;
        let area = Rect::new(0, 0, 30, 12);
        let (popup, buf) = paint(area, &viewer);
        let popup = popup.expect("popup");
        let top = row(&buf, popup, popup.y);
        assert!(
            top.starts_with("╭") && top.ends_with("╮"),
            "corners stay put around a wide title, got {top:?}"
        );
        // Wide glyphs occupy two cells; the second cell's symbol is blank,
        // so the joined row reads "画 面" rather than "画面".
        assert!(
            top.contains('画') && top.contains('面'),
            "the file name stays readable, got {top:?}"
        );
        assert!(
            top.contains('\u{2026}'),
            "a title wider than the popup is cut, got {top:?}"
        );
    }

    #[test]
    fn narrow_popup_still_shows_the_playback_clock() {
        let mut viewer = VideoViewerState::test_stub();
        viewer.playing = true;
        viewer.duration_secs = 65.0;
        viewer.video_width = 320;
        viewer.video_height = 240;
        let area = Rect::new(0, 0, 20, 10);
        let (popup, buf) = paint(area, &viewer);
        let popup = popup.expect("narrow popup");
        let bottom = row(&buf, popup, popup.y + popup.height - 1);
        assert!(
            bottom.starts_with("╰") && bottom.ends_with("╯"),
            "bottom corners stay put, got {bottom:?}"
        );
        assert!(
            bottom.contains("0:00"),
            "the clock stays on a narrow popup, got {bottom:?}"
        );
        assert!(
            UnicodeWidthStr::width(bottom.as_str()) <= popup.width as usize,
            "progress row ran past the popup: {bottom:?}"
        );
    }
}
