//! Prompt vim editing, active only when `[keys] vim` is `all`.
//!
//! Insert mode types as usual. Esc enters Normal. Normal has counts, motions
//! (`h` `l` `j` `k` `w` `b` `e` `0` `$`), the `d` operator (`dw`, `d$`, `dd`),
//! `x`, and `u` / Ctrl-R undo. `i` `a` `I` `A` return to Insert.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{PromptEvent, PromptWidget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum VimPromptMode {
    #[default]
    Insert,
    Normal,
}

impl VimPromptMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Insert => "INSERT",
            Self::Normal => "NORMAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PendingOp {
    #[default]
    None,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct VimPrompt {
    mode: VimPromptMode,
    count: u32,
    pending: PendingOp,
}

impl VimPrompt {
    pub(crate) fn mode(self) -> VimPromptMode {
        self.mode
    }
}

impl PromptWidget {
    /// `NORMAL` / `INSERT` when prompt vim editing is on. Otherwise the info line stays as it is.
    pub(crate) fn vim_prompt_info_label(&self) -> Option<&'static str> {
        vim_editing_enabled().then(|| self.vim_prompt.mode.label())
    }

    /// Push the vim mode onto the prompt info line. Bold is the minimal-mode row (`v71-f11`).
    pub(crate) fn append_vim_info_flag(&self, flags: &mut Vec<super::PromptFlag<'_>>) {
        if let Some(text) = self.vim_prompt_info_label() {
            flags.push(super::PromptFlag {
                text,
                color: None,
                bold: false,
            });
        }
    }

    /// `Some` when this key was a vim command. `None` means the normal insert path should run.
    pub(super) fn handle_vim_prompt_key(&mut self, key: &KeyEvent) -> Option<PromptEvent> {
        if !vim_editing_enabled() {
            return None;
        }
        if is_redo(key) && self.vim_prompt.mode == VimPromptMode::Normal {
            self.textarea.redo();
            self.vim_prompt.clear_pending();
            return Some(PromptEvent::Edited);
        }
        if !key.modifiers.is_empty() {
            return None;
        }
        match self.vim_prompt.mode {
            VimPromptMode::Insert => {
                if key.code == KeyCode::Esc {
                    self.vim_prompt.mode = VimPromptMode::Normal;
                    self.vim_prompt.clear_pending();
                    return Some(PromptEvent::Edited);
                }
                None
            }
            VimPromptMode::Normal => self.handle_vim_normal(key),
        }
    }

    fn handle_vim_normal(&mut self, key: &KeyEvent) -> Option<PromptEvent> {
        let Some(ch) = bare_char(key) else {
            if key.code == KeyCode::Esc {
                self.vim_prompt.clear_pending();
                return Some(PromptEvent::Edited);
            }
            // Arrows and other chords keep the textarea's behavior.
            return None;
        };
        if self.vim_prompt.pending == PendingOp::None && ch.is_ascii_digit() {
            if ch == '0' && self.vim_prompt.count == 0 {
                self.textarea.move_cursor_to_beginning_of_line(false);
                return Some(PromptEvent::Edited);
            }
            let digit = u32::from(ch as u8 - b'0');
            self.vim_prompt.count = self
                .vim_prompt
                .count
                .saturating_mul(10)
                .saturating_add(digit)
                .min(9999);
            return Some(PromptEvent::Edited);
        }
        let count = self.vim_prompt.count.max(1);
        self.vim_prompt.count = 0;
        match ch {
            'h' => self.apply_motion(count, Motion::Left),
            'l' => self.apply_motion(count, Motion::Right),
            'j' => self.apply_motion(count, Motion::Down),
            'k' => self.apply_motion(count, Motion::Up),
            'w' => self.apply_motion(count, Motion::Word),
            'b' => self.apply_motion(count, Motion::Back),
            'e' => self.apply_motion(count, Motion::End),
            '$' => self.apply_motion(count, Motion::EndOfLine),
            'x' => {
                self.vim_prompt.pending = PendingOp::None;
                self.textarea.delete_forward(count as usize);
            }
            'd' if self.vim_prompt.pending == PendingOp::Delete => {
                self.vim_prompt.pending = PendingOp::None;
                for _ in 0..count {
                    self.delete_line();
                }
            }
            'd' => {
                self.vim_prompt.pending = PendingOp::Delete;
                self.vim_prompt.count = count;
                return Some(PromptEvent::Edited);
            }
            'u' => {
                self.vim_prompt.pending = PendingOp::None;
                for _ in 0..count {
                    if !self.textarea.undo() {
                        break;
                    }
                }
            }
            'i' => self.enter_insert(false),
            'a' => {
                self.textarea.move_cursor_right();
                self.enter_insert(false);
            }
            'I' => {
                self.textarea.move_cursor_to_beginning_of_line(false);
                self.enter_insert(false);
            }
            'A' => {
                self.textarea.move_cursor_to_end_of_line(false);
                self.enter_insert(false);
            }
            _ => self.vim_prompt.pending = PendingOp::None,
        }
        Some(PromptEvent::Edited)
    }

    fn enter_insert(&mut self, _after: bool) {
        self.vim_prompt.mode = VimPromptMode::Insert;
        self.vim_prompt.clear_pending();
    }

    fn apply_motion(&mut self, count: u32, motion: Motion) {
        let deleting = self.vim_prompt.pending == PendingOp::Delete;
        self.vim_prompt.pending = PendingOp::None;
        if !deleting {
            for _ in 0..count {
                self.move_motion(motion);
            }
            return;
        }
        let start = self.textarea.cursor();
        for _ in 0..count {
            self.move_motion(motion);
        }
        let mut end = self.textarea.cursor();
        // `e` lands on the last character. A delete includes that character.
        if matches!(motion, Motion::End) && end < self.textarea.text().len() {
            let text = self.textarea.text().to_owned();
            end = char_after(&text, end);
        }
        let (from, to) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        if from == to {
            return;
        }
        self.textarea.set_cursor(from);
        self.textarea.replace_range(from..to, "");
    }

    fn move_motion(&mut self, motion: Motion) {
        match motion {
            Motion::Left => self.textarea.move_cursor_left(),
            Motion::Right => self.textarea.move_cursor_right(),
            Motion::Down => self.textarea.move_cursor_down(),
            Motion::Up => self.textarea.move_cursor_up(),
            Motion::EndOfLine => self.textarea.move_cursor_to_end_of_line(false),
            Motion::Word | Motion::Back | Motion::End => {
                let text = self.textarea.text().to_owned();
                let cursor = self.textarea.cursor();
                let next = match motion {
                    Motion::Word => word_forward(&text, cursor),
                    Motion::Back => word_back(&text, cursor),
                    Motion::End => word_end(&text, cursor),
                    _ => cursor,
                };
                self.textarea.set_cursor(next);
            }
        }
    }

    fn delete_line(&mut self) {
        let text = self.textarea.text().to_owned();
        let cursor = self.textarea.cursor();
        let start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let end = text[cursor..]
            .find('\n')
            .map(|i| cursor + i + 1)
            .unwrap_or(text.len());
        if start == end {
            return;
        }
        self.textarea.set_cursor(start);
        self.textarea.replace_range(start..end, "");
    }
}

impl VimPrompt {
    fn clear_pending(&mut self) {
        self.count = 0;
        self.pending = PendingOp::None;
    }
}

#[derive(Clone, Copy)]
enum Motion {
    Left,
    Right,
    Down,
    Up,
    Word,
    Back,
    End,
    EndOfLine,
}

fn vim_editing_enabled() -> bool {
    crate::appearance::cache::load_vim_keys() == crate::appearance::VimKeys::All
}

fn char_after(text: &str, pos: usize) -> usize {
    let pos = pos.min(text.len());
    match text[pos..].chars().next() {
        Some(ch) => pos + ch.len_utf8(),
        None => pos,
    }
}

fn bare_char(key: &KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(ch) if key.modifiers.is_empty() => Some(ch),
        _ => None,
    }
}

fn is_redo(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('r')) && key.modifiers == KeyModifiers::CONTROL
}

fn is_word(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn word_forward(text: &str, cursor: usize) -> usize {
    let chars: Vec<(usize, char)> = text.char_indices().filter(|(i, _)| *i >= cursor).collect();
    if chars.is_empty() {
        return text.len();
    }
    let mut i = 0;
    let wordish = is_word(chars[0].1);
    if wordish || !chars[0].1.is_whitespace() {
        while i < chars.len() && is_word(chars[i].1) == wordish && !chars[i].1.is_whitespace() {
            i += 1;
        }
    }
    while i < chars.len() && chars[i].1.is_whitespace() {
        i += 1;
    }
    chars.get(i).map(|(pos, _)| *pos).unwrap_or(text.len())
}

fn word_back(text: &str, cursor: usize) -> usize {
    let chars: Vec<(usize, char)> = text.char_indices().filter(|(i, _)| *i < cursor).collect();
    let mut i = chars.len();
    while i > 0 && chars[i - 1].1.is_whitespace() {
        i -= 1;
    }
    if i == 0 {
        return 0;
    }
    let wordish = is_word(chars[i - 1].1);
    while i > 0 && is_word(chars[i - 1].1) == wordish && !chars[i - 1].1.is_whitespace() {
        i -= 1;
    }
    chars.get(i).map(|(pos, _)| *pos).unwrap_or(0)
}

fn word_end(text: &str, cursor: usize) -> usize {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = chars
        .iter()
        .position(|(pos, _)| *pos >= cursor)
        .unwrap_or(chars.len());
    if i + 1 < chars.len() {
        i += 1;
    } else {
        return text.len();
    }
    while i < chars.len() && chars[i].1.is_whitespace() {
        i += 1;
    }
    if i >= chars.len() {
        return text.len();
    }
    let wordish = is_word(chars[i].1);
    while i + 1 < chars.len()
        && is_word(chars[i + 1].1) == wordish
        && !chars[i + 1].1.is_whitespace()
    {
        i += 1;
    }
    chars[i].0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_view::test_fixtures::make_agent;
    use crate::appearance::VimKeys;
    use crate::appearance::cache::{load_vim_keys, load_vim_mode, set_vim_keys};
    use crate::views::prompt_widget::PromptInfo;
    use crossterm::event::KeyEvent;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    struct KeysGuard(VimKeys);
    impl Drop for KeysGuard {
        fn drop(&mut self) {
            set_vim_keys(self.0);
        }
    }

    fn enable(keys: VimKeys) -> KeysGuard {
        let prev = load_vim_keys();
        set_vim_keys(keys);
        KeysGuard(prev)
    }

    fn key(ch: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
    }

    fn esc() -> KeyEvent {
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
    }

    #[test]
    fn all_mode_switches_and_shows_the_mode_on_the_info_line() {
        let _guard = enable(VimKeys::All);
        let mut agent = make_agent();
        agent.prompt.set_text("hello");
        agent.prompt.set_cursor(0);
        assert_eq!(agent.prompt.vim_prompt_info_label(), Some("INSERT"));
        assert_eq!(load_vim_keys(), VimKeys::All);
        assert!(load_vim_mode(), "all still navigates");

        let _ = agent.handle_prompt_key_for_test(&esc());
        assert_eq!(agent.prompt.vim_prompt_info_label(), Some("NORMAL"));
        assert_eq!(agent.prompt.text(), "hello", "Esc must not clear the draft");

        let _ = agent.handle_prompt_key_for_test(&key('x'));
        assert_eq!(agent.prompt.text(), "ello");
        assert_eq!(agent.prompt.vim_prompt_info_label(), Some("NORMAL"));

        let _ = agent.handle_prompt_key_for_test(&key('i'));
        assert_eq!(agent.prompt.vim_prompt_info_label(), Some("INSERT"));
        let _ = agent.handle_prompt_key_for_test(&key('Z'));
        assert_eq!(agent.prompt.text(), "Zello");

        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        let mut flags = Vec::new();
        agent.prompt.append_vim_info_flag(&mut flags);
        assert_eq!(flags.len(), 1);
        assert!(!flags[0].bold);
        agent.prompt.render_info_line(
            &mut buf,
            area,
            &PromptInfo {
                flags: &flags,
                ..Default::default()
            },
            ratatui::style::Color::Reset,
            &crate::theme::Theme::current(),
            true,
        );
        let painted: String = (0..area.width)
            .filter_map(|x| buf.cell((x, 0)).map(|cell| cell.symbol()))
            .collect();
        assert!(
            painted.contains("INSERT"),
            "the info line must show the mode, got {painted:?}"
        );
    }

    #[test]
    fn normal_mode_count_motion_operator_and_undo() {
        let _guard = enable(VimKeys::All);
        let mut agent = make_agent();
        agent.prompt.set_text("one two three");
        agent.prompt.set_cursor(0);
        let _ = agent.handle_prompt_key_for_test(&esc());

        let _ = agent.handle_prompt_key_for_test(&key('2'));
        let _ = agent.handle_prompt_key_for_test(&key('l'));
        assert_eq!(agent.prompt.cursor(), 2, "2l moves two characters");

        let _ = agent.handle_prompt_key_for_test(&key('0'));
        assert_eq!(agent.prompt.cursor(), 0);
        let _ = agent.handle_prompt_key_for_test(&key('d'));
        let _ = agent.handle_prompt_key_for_test(&key('w'));
        assert_eq!(agent.prompt.text(), "two three");

        let _ = agent.handle_prompt_key_for_test(&key('u'));
        assert_eq!(agent.prompt.text(), "one two three");
        assert_eq!(agent.prompt.vim_prompt_info_label(), Some("NORMAL"));
    }

    #[test]
    fn nav_does_not_enter_prompt_vim() {
        let _guard = enable(VimKeys::Nav);
        let mut agent = make_agent();
        agent.prompt.set_text("hello");
        agent.prompt.set_cursor(agent.prompt.text().len());
        assert!(load_vim_mode());
        assert_eq!(agent.prompt.vim_prompt_info_label(), None);
        let _ = agent.handle_prompt_key_for_test(&esc());
        let _ = agent.handle_prompt_key_for_test(&key('x'));
        assert_eq!(
            agent.prompt.text(),
            "hellox",
            "nav keeps letters as insert text"
        );
    }
}
