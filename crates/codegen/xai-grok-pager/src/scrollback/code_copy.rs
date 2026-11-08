//! A `[Copy]` row under a fenced code block in a finished reply.
//! The label is the fence language, or the file name of a `start:end:path` citation.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::scrollback::types::{BlockLine, BlockOutput};
use crate::theme::Theme;
use xai_grok_markdown::CodeBlockSpan;

/// The button row under one code block, and the text a click copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCopyAffordance {
    /// Index of the button row in the block's `output()` lines.
    pub row_offset: u16,
    /// Painted label, `[Copy]` plus the language or file name when there is one.
    pub label: String,
    /// Fence body. This is what the button copies.
    pub body: String,
}

impl CodeCopyAffordance {
    pub fn clipboard_text(&self) -> &str {
        &self.body
    }
}

/// `[Copy]`, `[Copy] rust`, or `[Copy] main.rs`.
pub fn code_copy_label(info: &str) -> String {
    match code_copy_name(info) {
        Some(name) => format!("[Copy] {name}"),
        None => "[Copy]".to_string(),
    }
}

/// Language token, or the file name of a line-range citation. Empty info has neither.
fn code_copy_name(info: &str) -> Option<String> {
    let info = info.trim();
    if info.is_empty() {
        return None;
    }
    if let Some(path) = citation_path(info) {
        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path);
        return Some(name.to_string());
    }
    info.split_whitespace().next().map(str::to_string)
}

/// `10:20:src/main.rs` → `src/main.rs`. The same shape the highlighter treats as a citation.
fn citation_path(info: &str) -> Option<&str> {
    let mut parts = info.splitn(3, ':');
    let start = parts.next()?;
    let end = parts.next()?;
    let path = parts.next()?;
    if start.is_empty() || !start.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    if end.is_empty() || !end.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    if path.is_empty() {
        return None;
    }
    Some(path)
}

pub fn is_mermaid_fence(info: &str) -> bool {
    info.split_whitespace()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("mermaid"))
}

fn button_line(label: &str) -> BlockLine {
    let theme = Theme::current();
    let style = Style::default().fg(theme.gray);
    BlockLine::separator(Line::from(Span::styled(label.to_string(), style)))
        .with_joiner(Some(String::new()))
}

/// Exclusive end row of each pre-wrap line, after wrapping.
fn prewrap_end_rows(lines: &[BlockLine]) -> Vec<usize> {
    let mut ends = Vec::new();
    for (row, prewrap) in crate::scrollback::types::prewrap_index_per_row(lines)
        .into_iter()
        .enumerate()
    {
        if let Some(slot) = ends.get_mut(prewrap) {
            *slot = row + 1;
        } else {
            ends.push(row + 1);
        }
    }
    ends
}

/// Insert a non-selectable `[Copy]` row after each code block.
/// The row is not part of the message copy. A mermaid fence keeps the diagram's own buttons.
pub fn apply_code_copy_rows(
    output: &mut BlockOutput,
    spans: &[CodeBlockSpan],
) -> Vec<CodeCopyAffordance> {
    if spans.is_empty() || output.lines.is_empty() {
        return Vec::new();
    }
    let ends = prewrap_end_rows(&output.lines);
    let mut inserts: Vec<(usize, String, String)> = Vec::new();
    for span in spans {
        if span.output_line_range.is_empty() || is_mermaid_fence(&span.info) {
            continue;
        }
        let Some(&insert_at) = ends.get(span.output_line_range.end - 1) else {
            continue;
        };
        inserts.push((
            insert_at,
            code_copy_label(&span.info),
            span.body.clone(),
        ));
    }
    let affordances = inserts
        .iter()
        .enumerate()
        .map(|(index, (at, label, body))| CodeCopyAffordance {
            row_offset: at.saturating_add(index) as u16,
            label: label.clone(),
            body: body.clone(),
        })
        .collect();
    for (at, label, _) in inserts.iter().rev() {
        output.lines.insert(*at, button_line(label));
    }
    affordances
}

/// A visible code-copy row and the text its button copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCopyPlacement {
    pub screen_rect: ratatui::layout::Rect,
    pub label: String,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_copy_label_uses_the_language_or_the_file_name() {
        assert_eq!(code_copy_label(""), "[Copy]");
        assert_eq!(code_copy_label("   "), "[Copy]");
        assert_eq!(code_copy_label("rust"), "[Copy] rust");
        assert_eq!(code_copy_label("python extra"), "[Copy] python");
        assert_eq!(code_copy_label("10:20:src/lib.rs"), "[Copy] lib.rs");
        assert_eq!(
            code_copy_label("1:2:crates/foo/bar.rs"),
            "[Copy] bar.rs"
        );
        assert!(!is_mermaid_fence("rust"));
        assert!(is_mermaid_fence("mermaid theme=base"));
    }
}
