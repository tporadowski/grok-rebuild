use super::*;
use crossterm::event::KeyModifiers;

fn editable_state() -> (tempfile::TempDir, PathBuf, PersonaDetailState) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("reviewer.toml");
    std::fs::write(
        &path,
        concat!(
            "name = \"reviewer\"\n",
            "description = \"old description\"\n",
            "model = \"grok\"\n",
            "reasoning_effort = \"high\"\n",
            "default_isolation = \"worktree\"\n",
            "instructions = \"read only instructions\"\n",
        ),
    )
    .unwrap();
    let state = PersonaDetailState::from_toml_file(&path, true, "project").unwrap();
    (directory, path, state)
}

#[test]
fn detail_edit_save_updates_state_and_toml() {
    let (_directory, path, mut state) = editable_state();
    state.selected_field = PersonaField::Description;
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
    );
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
    );
    for ch in "new description".chars() {
        let _ = handle_persona_detail_key(
            &mut state,
            &KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        );
    }
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );

    assert!(!state.is_editing());
    assert_eq!(state.description, "new description");
    assert!(state.dirty);
    let saved = std::fs::read_to_string(path).unwrap();
    assert!(saved.contains("description = \"new description\""));
}

#[test]
fn detail_edit_cancel_preserves_original_and_file() {
    let (_directory, path, mut state) = editable_state();
    let before = std::fs::read_to_string(&path).unwrap();
    state.selected_field = PersonaField::Name;
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE),
    );
    let _ = handle_persona_detail_key(&mut state, &KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(!state.is_editing());
    assert_eq!(state.name, "reviewer");
    assert!(!state.dirty);
    assert_eq!(std::fs::read_to_string(path).unwrap(), before);
}

#[test]
fn detail_unchanged_edit_does_not_write_or_mark_dirty() {
    let (_directory, path, mut state) = editable_state();
    let before = std::fs::read_to_string(&path).unwrap();
    state.selected_field = PersonaField::Model;
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );

    assert!(!state.is_editing());
    assert!(!state.dirty);
    assert!(state.message.is_none());
    assert_eq!(std::fs::read_to_string(path).unwrap(), before);
}

#[test]
fn multiline_values_require_source_file_editing() {
    let (_directory, _path, mut state) = editable_state();
    state.description = "first line\nsecond line".to_owned();
    state.selected_field = PersonaField::Description;

    let outcome = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert!(matches!(outcome, PersonaDetailOutcome::Changed));
    assert!(!state.is_editing());
    assert_eq!(state.description, "first line\nsecond line");
    assert_eq!(
        state.message.as_deref(),
        Some("Multiline values must be edited in the source file")
    );
}

#[test]
fn detail_instructions_remain_read_only_inline() {
    let (_directory, _path, mut state) = editable_state();
    state.selected_field = PersonaField::Instructions;
    let outcome = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert!(matches!(outcome, PersonaDetailOutcome::Changed));
    assert!(!state.is_editing());
    assert!(state.instructions_expanded);
}

#[test]
fn detail_paste_targets_only_active_editor_and_sanitizes() {
    let (_directory, _path, mut state) = editable_state();
    state.selected_field = PersonaField::Model;
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    state.set_editing_text("ab");
    let _ = state.set_editing_cursor_byte(1);
    let outcome = handle_persona_detail_paste(&mut state, "中\r\n");
    assert!(matches!(outcome, PersonaDetailOutcome::Changed));
    assert_eq!(state.editing_text(), Some("a中b"));

    let _ = handle_persona_detail_key(&mut state, &KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let outcome = handle_persona_detail_paste(&mut state, "ignored");
    assert!(matches!(outcome, PersonaDetailOutcome::Unchanged));
}

#[test]
fn detail_editor_uses_canonical_graphemes_and_keeps_cursor_visible() {
    let (_directory, _path, mut state) = editable_state();
    state.selected_field = PersonaField::Model;
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    let grapheme = "👩🏽\u{200d}💻";
    state.set_editing_text(format!("a{grapheme}b"));
    let _ = state.set_editing_cursor_byte(1);
    let _ = handle_persona_detail_key(
        &mut state,
        &KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
    );
    assert_eq!(state.editing_text(), Some("ab"));

    let text = format!("123456中e\u{301}{grapheme}z");
    state.set_editing_text(&text);
    let _ = state.set_editing_cursor_byte(text.len() - 1);

    let width = 10usize;
    let theme = Theme::current();
    let mut buffer = Buffer::empty(Rect::new(0, 0, width as u16, 1));
    let Some(viewport) = state.editing_viewport(width) else {
        panic!("expected editing viewport");
    };
    let Some(text) = state.editing_text() else {
        panic!("expected editing text");
    };
    let Some(visible) = text.get(viewport.visible_byte_range.clone()) else {
        panic!("viewport out of range");
    };
    assert!(visible.contains('中'));
    assert!(visible.contains("e\u{301}"));
    assert!(visible.contains(grapheme));
    let Some(editor) = state.editing_editor() else {
        panic!("expected editing editor");
    };
    render_detail_editor(&mut buffer, 0, 0, width, editor, Style::default(), &theme);
    let cursor_x = viewport.cursor_display_column as u16;
    assert_eq!(
        buffer.cell((cursor_x, 0)).map(|c| c.bg),
        Some(theme.text_primary)
    );
}

#[test]
fn browse_footer_advertises_arrow_nav() {
    let (_directory, _path, state) = editable_state();
    assert!(!state.is_editing());
    let labels: Vec<_> = build_shortcuts(&state).iter().map(|sc| sc.label).collect();
    assert!(
        labels.contains(&"\u{2191}/\u{2193} nav"),
        "persona detail footer must advertise arrow navigation: {labels:?}"
    );
    assert!(
        labels.iter().all(|label| !label.contains("j/k")),
        "persona detail footer must not advertise j/k: {labels:?}"
    );
}

fn row_text(buf: &Buffer, y: u16, x: u16, width: u16) -> String {
    (x..x + width)
        .filter_map(|col| buf.cell((col, y)).map(|cell| cell.symbol().to_string()))
        .collect()
}

fn assert_row_background(buf: &Buffer, y: u16, x: u16, width: u16, bg: ratatui::style::Color) {
    for col in x..x + width {
        let cell = buf.cell((col, y)).unwrap_or_else(|| panic!("cell {col},{y}"));
        assert_eq!(cell.bg, bg, "column {col} of row {y} lost the selection fill");
    }
}

/// Content origin of a non-compact persona detail modal in an 80-column area.
/// Each single-line field occupies its row and then skips one, so Instructions
/// starts ten rows below the first field.
fn persona_detail_content(area_height: u16) -> (u16, u16, u16) {
    let area_width = 80u16;
    let max_width = area_width.saturating_sub(4).min(100);
    let preferred = (area_width as f32 * 0.70) as u16;
    let modal_width = preferred.min(max_width).max(44).min(area_width);
    let modal_height = area_height.saturating_sub(8);
    let modal_x = (area_width - modal_width) / 2;
    let modal_y = (area_height - modal_height) / 2;
    let content_x = modal_x + 1 + 2;
    let content_y = modal_y + 1 + 1;
    let content_w = modal_width - 2 - 4;
    (content_x, content_y, content_w)
}

#[test]
#[serial_test::serial]
fn selected_multiline_field_highlights_every_row_and_counts_hidden_lines() {
    let _guard = crate::theme::cache::pin_theme();
    crate::theme::cache::set_terminal_native_lock(false);
    crate::theme::cache::set(crate::theme::ThemeKind::TokyoNight);
    let was_embedded = crate::views::modal_window::embedded();
    crate::views::modal_window::set_embedded(false);
    let theme = Theme::current();
    let highlight = theme.bg_highlight;

    let paint = |state: &mut PersonaDetailState, height: u16| {
        let area = Rect::new(0, 0, 80, height);
        let mut buf = Buffer::empty(area);
        render_persona_detail(&mut buf, area, state, &theme, false);
        buf
    };

    // Short modal: 12 instruction lines, only 3 fit, so the hint says 9.
    let mut state = PersonaDetailState::from_name_only("reviewer");
    state.instructions = (0..12)
        .map(|i| format!("alpha-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    state.selected_field = PersonaField::Instructions;
    let buf = paint(&mut state, 27);
    let (content_x, content_y, content_w) = persona_detail_content(27);
    let instr_y = content_y + 10;
    assert_eq!(
        row_text(&buf, instr_y, content_x, content_w).trim(),
        "Instructions  alpha-00",
    );
    assert!(row_text(&buf, instr_y + 1, content_x, content_w).contains("alpha-01"));
    assert!(row_text(&buf, instr_y + 2, content_x, content_w).contains("alpha-02"));
    let hint = row_text(&buf, instr_y + 3, content_x, content_w);
    assert!(
        hint.contains("(9 more lines: e to expand, j/k to scroll)"),
        "collapsed hint should count lines the viewport hid, got {hint:?}"
    );
    assert!(
        !hint.contains("4 more lines"),
        "hint must not use total - 8 when only 3 lines were shown"
    );
    for row in instr_y..instr_y + 4 {
        assert_row_background(&buf, row, content_x, content_w, highlight);
    }
    assert_ne!(
        buf.cell((content_x + content_w - 1, content_y))
            .map(|cell| cell.bg),
        Some(highlight),
        "the unselected name row is not part of the selection"
    );

    state.instructions_expanded = true;
    let buf = paint(&mut state, 27);
    assert!(row_text(&buf, instr_y + 1, content_x, content_w).contains("alpha-01"));
    let expanded_hint = row_text(&buf, instr_y + 3, content_x, content_w);
    assert!(
        expanded_hint.contains("(e to collapse, j/k to scroll"),
        "expanded instructions keep their scroll hint, got {expanded_hint:?}"
    );
    for row in instr_y..instr_y + 4 {
        assert_row_background(&buf, row, content_x, content_w, highlight);
    }

    // Tall modal: the collapsed cap is 8, so 10 lines leave 2 hidden.
    let mut tall = PersonaDetailState::from_name_only("reviewer");
    tall.instructions = (0..10)
        .map(|i| format!("beta-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    tall.selected_field = PersonaField::Instructions;
    let buf = paint(&mut tall, 40);
    let (content_x, content_y, content_w) = persona_detail_content(40);
    let instr_y = content_y + 10;
    let hint = row_text(&buf, instr_y + 8, content_x, content_w);
    assert!(
        hint.contains("(2 more lines: e to expand, j/k to scroll)"),
        "eight shown lines of ten should leave two, got {hint:?}"
    );
    assert!(row_text(&buf, instr_y + 7, content_x, content_w).contains("beta-07"));

    // A wrapped single field highlights every continuation row.
    let mut wrapped = PersonaDetailState::from_name_only("reviewer");
    wrapped.description = (0..12)
        .map(|i| format!("wrap{i:02}"))
        .collect::<Vec<_>>()
        .join(" ");
    wrapped.selected_field = PersonaField::Description;
    let buf = paint(&mut wrapped, 32);
    let (content_x, content_y, content_w) = persona_detail_content(32);
    let desc_y = content_y + 2;
    let first = row_text(&buf, desc_y, content_x, content_w);
    let second = row_text(&buf, desc_y + 1, content_x, content_w);
    let third = row_text(&buf, desc_y + 2, content_x, content_w);
    assert!(first.contains("wrap00") && first.contains("wrap04"), "{first:?}");
    assert!(
        second.contains("wrap05") && second.contains("wrap09"),
        "{second:?}"
    );
    assert!(third.contains("wrap10") && third.contains("wrap11"), "{third:?}");
    for row in desc_y..desc_y + 3 {
        assert_row_background(&buf, row, content_x, content_w, highlight);
    }

    crate::views::modal_window::set_embedded(was_embedded);
}
