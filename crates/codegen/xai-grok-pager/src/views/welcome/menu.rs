use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::render::SafeBuf;
use crate::theme::Theme;

use super::logo::logo_visual_width;

fn cols(text: &str) -> u16 {
    unicode_width::UnicodeWidthStr::width(text) as u16
}

/// Render the welcome menu rows as `label … shortcut`, padded within each row.
/// Returns the Rect for each item row (for hit-testing clicks and hover).
pub fn render_menu(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    items: &[(&str, &str)],
    selected: Option<usize>,
    mouse_pos: Option<(u16, u16)>,
    min_width_hint: u16,
) -> Vec<Rect> {
    let label_style = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD);
    let label_selected_style = Style::default()
        .fg(theme.text_primary)
        .bg(theme.bg_highlight)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(theme.gray_bright);
    let key_selected_style = Style::default()
        .fg(theme.gray_bright)
        .bg(theme.bg_highlight);

    // Width: label + gap + key
    // Keep a 4-col gap between label and key for readability
    let content_min: u16 = items
        .iter()
        .map(|(key, label)| cols(key) + cols(label) + 4)
        .max()
        .unwrap_or(0);
    let menu_width = logo_visual_width(area.height)
        .max(30)
        .max(content_min)
        .max(min_width_hint);

    let [_, menu_centered, _] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(menu_width),
        Constraint::Min(0),
    ])
    .flex(Flex::Center)
    .areas(area);

    let mut rects = Vec::with_capacity(items.len());
    let mut y = menu_centered.y;
    for (i, (key, label)) in items.iter().enumerate() {
        if y >= menu_centered.y.saturating_add(menu_centered.height) {
            break;
        }

        let is_selected = selected == Some(i);
        let row_width = menu_centered.width;
        // A zero-width row cannot hold a shortcut. Keep the index, skip the paint.
        if row_width == 0 {
            rects.push(Rect {
                x: menu_centered.x,
                y,
                width: 0,
                height: 0,
            });
            y = y.saturating_add(1);
            continue;
        }
        let key_width = cols(key);
        // A shortcut wider than the row used to underflow. Clip it onto the row.
        let key_room = key_width.min(row_width);
        let key_drawn = if key_width == 0 || key_room == key_width {
            (*key).to_string()
        } else {
            crate::render::line_utils::truncate_str(key, key_room as usize)
        };
        let key_drawn_width = cols(&key_drawn);
        let key_x = menu_centered
            .x
            .saturating_add(row_width.saturating_sub(key_drawn_width));
        let label_budget = key_x.saturating_sub(menu_centered.x).saturating_sub(1);
        let label = crate::render::line_utils::truncate_str(label, label_budget as usize);
        let label_len = cols(&label);

        let row_rect = Rect {
            x: menu_centered.x,
            y,
            width: menu_centered.width,
            height: 1,
        };
        rects.push(row_rect);

        // Fill row background when selected/hovered
        if is_selected {
            let hover_bg = Style::default().bg(theme.bg_highlight);
            let row_right = menu_centered.x.saturating_add(menu_centered.width);
            for x in menu_centered.x..row_right {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(hover_bg);
                }
            }
        }

        // Label, flush with the left edge of the menu column.
        let lstyle = if is_selected {
            label_selected_style
        } else {
            label_style
        };
        if label_len > 0 {
            buf.set_span_safe(
                menu_centered.x,
                y,
                &Span::styled(&*label, lstyle),
                label_len,
            );
        }

        // Key shortcut flush with the right edge of the menu column.
        let kstyle = if is_selected {
            key_selected_style
        } else {
            key_style
        };
        if key_drawn_width > 0 {
            buf.set_span_safe(
                key_x,
                y,
                &Span::styled(key_drawn.as_str(), kstyle),
                key_drawn_width,
            );
        }

        // Restyle the [x] dismiss control (for the import row). A clipped key no longer holds it.
        if key_room == key_width
            && let Some(x_offset) = key.rfind("[x]")
        {
            let key_x_start = key_x;
            let dismiss_start = key_x_start.saturating_add(x_offset as u16);
            let dismiss_end = dismiss_start + 3;
            let mouse_on_dismiss = mouse_pos
                .is_some_and(|(mx, my)| my == y && mx >= dismiss_start && mx < dismiss_end);
            let dismiss_color = if mouse_on_dismiss {
                theme.text_primary
            } else {
                theme.gray_bright
            };
            let dismiss_style = if is_selected {
                Style::default()
                    .fg(dismiss_color)
                    .bg(theme.bg_highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(dismiss_color)
                    .add_modifier(Modifier::BOLD)
            };
            for (offset, ch) in "[x]".chars().enumerate() {
                let col = dismiss_start + offset as u16;
                if let Some(cell) = buf.cell_mut((col, y)) {
                    cell.set_char(ch);
                    cell.set_style(dismiss_style);
                }
            }
        }

        // Terminal theme (Reset band slots): the selection/hover cue is
        // reverse video; the row is all default-fg text so it inverts
        // uniformly. RGB themes keep their bg_highlight band.
        if is_selected && theme.is_bandless() {
            buf.set_style(row_rect, Style::default().add_modifier(Modifier::REVERSED));
        }

        y = y.saturating_add(1);
    }

    rects
}
