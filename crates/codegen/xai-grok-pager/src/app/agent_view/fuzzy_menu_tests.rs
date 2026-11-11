//! Prompt history search and the `/` / `@` menus share one fuzzy highlight and one dim border caption.
use super::{AgentView, AppRenderParams, BannerSlotParams, test_fixtures};
use crate::actions::ActionRegistry;
use crate::scrollback::render::ScratchBuffer;
use crate::views::history_search::HistoryEntry;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

fn draw(agent: &mut AgentView) -> Buffer {
    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    let mut scratch = ScratchBuffer::new();
    agent.draw(
        area,
        &mut buf,
        &ActionRegistry::defaults(),
        &mut scratch,
        None,
        false,
        BannerSlotParams::none(),
        false,
        &mut Vec::new(),
        AppRenderParams::default(),
    );
    buf
}

fn rows(buf: &Buffer) -> String {
    (buf.area.y..buf.area.bottom())
        .map(|y| {
            (buf.area.x..buf.area.right())
                .filter_map(|x| buf.cell((x, y)).map(|cell| cell.symbol().to_string()))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Caption glyphs on the right of a dropdown border (`12`, `3/40`), skipping the `─` rule.
fn right_border_caption(buf: &Buffer, y: u16) -> Vec<(String, Color, Color, Modifier)> {
    let mut cells = Vec::new();
    let mut started = false;
    for x in (buf.area.x..buf.area.right()).rev() {
        let Some(cell) = buf.cell((x, y)) else {
            continue;
        };
        let symbol = cell.symbol();
        let caption = !symbol.is_empty()
            && symbol
                .chars()
                .all(|ch| ch.is_ascii_digit() || ch == '/');
        if caption {
            started = true;
            cells.push((
                symbol.to_string(),
                cell.fg,
                cell.bg,
                cell.modifier,
            ));
            continue;
        }
        if started {
            break;
        }
    }
    cells.reverse();
    cells
}

fn assert_dim_caption(buf: &Buffer, y: u16, expected_text: &str) {
    let theme = crate::theme::Theme::current();
    assert!(theme.is_bandless(), "dim captions show up on the terminal palette");
    let expected = theme.muted_over_chrome().bg(theme.bg_base);
    assert!(
        expected.add_modifier.contains(Modifier::DIM),
        "terminal captions dim instead of inheriting the border fg"
    );
    let cells = right_border_caption(buf, y);
    let text: String = cells.iter().map(|(symbol, _, _, _)| symbol.as_str()).collect();
    assert_eq!(text, expected_text, "border row:\n{}", rows(buf));
    let expected_fg = expected.fg.unwrap_or(Color::Reset);
    let expected_bg = expected.bg.unwrap_or(Color::Reset);
    for (symbol, fg, bg, modifier) in &cells {
        assert_eq!(*fg, expected_fg, "caption {symbol} fg");
        assert_eq!(*bg, expected_bg, "caption {symbol} bg");
        assert!(
            modifier.contains(Modifier::DIM),
            "caption {symbol} should be dimmed, modifier={modifier:?}"
        );
    }
}

fn with_terminal_theme(body: impl FnOnce()) {
    let _guard = crate::theme::cache::pin_theme();
    crate::theme::cache::set(crate::theme::ThemeKind::Terminal);
    let was_embedded = crate::views::modal_window::embedded();
    crate::views::modal_window::set_embedded(false);
    body();
    crate::views::modal_window::set_embedded(was_embedded);
    crate::theme::cache::set(crate::theme::ThemeKind::GrokNight);
}

#[test]
fn history_search_highlights_matches_with_fuzzy_accent() {
    let _guard = crate::theme::cache::pin_theme();
    let theme = crate::theme::Theme::current();
    assert_ne!(
        theme.fuzzy_accent, theme.accent_user,
        "the fuzzy color has to differ from the old history highlight or this test cannot see the fix"
    );

    let mut agent = test_fixtures::make_agent();
    let history = vec![HistoryEntry {
        text: "ship the cargo fix".to_string(),
    }];
    assert!(agent.prompt.history_search.activate(&history, ""));
    agent.prompt.history_search.update_query("cargo");
    let mut highlighted = false;
    for _ in 0..200 {
        agent.prompt.history_search.poll();
        highlighted = agent
            .prompt
            .history_search
            .result_at(agent.prompt.history_search.selected)
            .is_some_and(|row| row.text.contains("cargo") && !row.indices.is_empty());
        if highlighted {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(highlighted, "history matcher did not highlight cargo");
    assert_eq!(agent.prompt.history_search.result_count(), 1);

    let selected = agent.prompt.history_search.selected;
    let row = agent
        .prompt
        .history_search
        .result_at(selected)
        .expect("highlighted row")
        .clone();
    let buf = draw(&mut agent);
    let items = agent.history_dropdown_area.unwrap_or_else(|| {
        panic!("history panel missing:\n{}", rows(&buf));
    });
    let y = items.y.saturating_add(selected as u16);
    let mut col = items.x.saturating_add(crate::glyphs::PROMPT_ARROW_WIDTH);
    let matched: std::collections::HashSet<u32> = row.indices.iter().copied().collect();
    for (ci, ch) in row.text.chars().enumerate() {
        let cell = buf.cell((col, y)).unwrap_or_else(|| {
            panic!("missing history cell ({col},{y}) for {ch}:\n{}", rows(&buf))
        });
        assert_eq!(cell.symbol(), ch.to_string(), "history row:\n{}", rows(&buf));
        let fg = if matched.contains(&(ci as u32)) {
            theme.fuzzy_accent
        } else {
            theme.text_primary
        };
        assert_eq!(cell.fg, fg, "history char {ch} at {ci}");
        let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        col = col.saturating_add(width);
    }
}

#[test]
fn slash_menu_border_count_is_dimmed_like_other_captions() {
    with_terminal_theme(|| {
        let mut agent = test_fixtures::make_agent();
        agent.prompt.set_text("/");
        agent.prompt.refresh_slash(&agent.session.models);
        assert!(agent.prompt.slash_open(), "slash menu should open on `/`");
        let count = agent.prompt.slash_snapshot().matches.len();
        assert!(count > 0, "slash menu should list commands");
        let buf = draw(&mut agent);
        let items = agent.slash_dropdown_items_area.unwrap_or_else(|| {
            panic!("slash panel missing:\n{}", rows(&buf));
        });
        let y = items.y.checked_sub(1).expect("slash border above the items");
        assert_dim_caption(&buf, y, &count.to_string());
    });
}

#[test]
fn at_menu_border_count_is_dimmed_like_other_captions() {
    with_terminal_theme(|| {
        let mut agent = test_fixtures::make_agent();
        let ctx = crate::views::file_search::context::detect("@src", 4).expect("@ context");
        agent.prompt.file_search.set_test_state(
            ctx,
            vec![xai_grok_workspace::file_system::FuzzyMatchResult {
                path: nucleo::Utf32String::from("src/lib.rs"),
                ..Default::default()
            }],
            0,
        );
        assert!(agent.prompt.file_search_visible());
        let k = agent.prompt.file_search.result_count();
        let n = agent.prompt.file_search.total_items();
        let expected = if k >= 1000 {
            format!("1k+/{n}")
        } else {
            format!("{k}/{n}")
        };
        let buf = draw(&mut agent);
        let items = agent.dropdown_items_area.unwrap_or_else(|| {
            panic!("@ panel missing:\n{}", rows(&buf));
        });
        let y = items.y.checked_sub(1).expect("@ border above the items");
        assert_dim_caption(&buf, y, &expected);
    });
}
