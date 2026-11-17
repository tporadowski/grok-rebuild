use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::render::SafeBuf;
use crate::theme::Theme;

use super::logo::logo_visual_width;

fn cols(text: &str) -> u16 {
    unicode_width::UnicodeWidthStr::width(text) as u16
}

/// Background for a selected or hovered welcome row.
/// Light themes lift the palette band toward white. Dark themes keep it.
pub(super) fn welcome_row_bg(theme: &Theme, selected: bool, hovered: bool) -> Option<Color> {
    if !selected && !hovered {
        return None;
    }
    let raw = if hovered {
        theme.bg_hover
    } else {
        theme.bg_highlight
    };
    Some(if theme.is_dark() {
        raw
    } else {
        lift_light_band(raw)
    })
}

fn lift_light_band(color: Color) -> Color {
    let Color::Rgb(r, g, b) = color else {
        return color;
    };
    let lift = |channel: u8| channel.saturating_add(24).min(248);
    Color::Rgb(lift(r), lift(g), lift(b))
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
    let key_style = Style::default().fg(theme.gray_bright);

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
        let row_right = menu_centered.x.saturating_add(row_width);
        let hovered = mouse_pos.is_some_and(|(mx, my)| {
            my == y && mx >= menu_centered.x && mx < row_right
        });
        let band = welcome_row_bg(theme, is_selected, hovered);
        let emphasized = is_selected || hovered;
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

        // Fill row background when selected or hovered.
        if let Some(bg) = band {
            let hover_bg = Style::default().bg(bg);
            for x in menu_centered.x..row_right {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(hover_bg);
                }
            }
        }

        // Label, flush with the left edge of the menu column.
        let lstyle = if let Some(bg) = band {
            label_style.bg(bg)
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
        let kstyle = if let Some(bg) = band {
            key_style.bg(bg)
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
            let dismiss_style = if let Some(bg) = band {
                Style::default()
                    .fg(dismiss_color)
                    .bg(bg)
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
        if emphasized && theme.is_bandless() {
            buf.set_style(row_rect, Style::default().add_modifier(Modifier::REVERSED));
        }

        y = y.saturating_add(1);
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    fn channel_sum(color: Color) -> i32 {
        let Color::Rgb(r, g, b) = color else {
            panic!("welcome band must stay rgb, got {color:?}");
        };
        i32::from(r) + i32::from(g) + i32::from(b)
    }

    #[test]
    #[serial_test::serial]
    fn light_welcome_rows_use_lighter_selection_and_hover() {
        let _guard = crate::theme::cache::pin_theme();
        crate::theme::cache::set_terminal_native_lock(false);
        crate::theme::cache::set(crate::theme::ThemeKind::GrokDay);
        let theme = Theme::current();
        assert!(!theme.is_dark());
        let source = Theme::grokday();
        let area = Rect::new(0, 0, 40, 4);
        let items = [("a", "Resume"), ("b", "Quit"), ("c", "New")];
        let mut buf = Buffer::empty(area);
        render_menu(
            area,
            &mut buf,
            &theme,
            &items,
            Some(0),
            Some((20, 1)),
            0,
        );
        let selected = buf.cell((20, 0)).expect("selected row").bg;
        let hovered = buf.cell((20, 1)).expect("hovered row").bg;
        let idle = buf.cell((20, 2)).expect("idle row").bg;
        assert!(
            channel_sum(selected) > channel_sum(source.bg_highlight),
            "selection {selected:?} should be lighter than {:?}",
            source.bg_highlight
        );
        assert!(
            channel_sum(hovered) > channel_sum(source.bg_hover),
            "hover {hovered:?} should be lighter than {:?}",
            source.bg_hover
        );
        assert_ne!(selected, hovered);
        assert_ne!(idle, selected);
        assert_ne!(idle, hovered);

        crate::theme::cache::set(crate::theme::ThemeKind::TokyoNight);
        let dark = Theme::current();
        assert!(dark.is_dark());
        let mut buf = Buffer::empty(area);
        render_menu(area, &mut buf, &dark, &items, Some(0), None, 0);
        assert_eq!(buf.cell((20, 0)).expect("dark selected").bg, dark.bg_highlight);
    }
}
