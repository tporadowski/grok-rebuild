//! Renders with a golden accent line.
//! Collapsed (default) shows a single `/btw <question>` header line; expanded shows the full markdown response below the header.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::scrollback::block::BlockContent;
use crate::scrollback::code_copy::CodeCopyAffordance;
use crate::scrollback::types::{AccentStyle, BlockContext, BlockLine, BlockOutput, DisplayMode};
use crate::theme::Theme;

use super::markdown_content::MarkdownContent;
use crate::appearance::AppearanceConfig;

/// Block displaying a /btw side-question and its response.
#[derive(Debug, Clone)]
pub struct BtwBlock {
    /// The original question text.
    pub question: String,
    /// Rendered response content (markdown).
    content: MarkdownContent,
}

impl BtwBlock {
    pub fn new(question: impl Into<String>, response: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            content: MarkdownContent::new(response),
        }
    }

    pub fn content(&self) -> &MarkdownContent {
        &self.content
    }
}

impl BlockContent for BtwBlock {
    fn output(&self, ctx: &BlockContext) -> BlockOutput {
        let theme = Theme::current();
        let is_collapsed = ctx.mode == DisplayMode::Collapsed;
        let tool_cfg = &ctx.appearance.scrollback.blocks.tool;
        let is_muted = tool_cfg.muted_collapsed && is_collapsed;

        // Header: "/btw <question>"
        let header_style = if is_muted {
            theme.muted().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(theme.accent_plan)
                .add_modifier(Modifier::BOLD)
        };
        let header = Line::from(Span::styled(
            format!("/btw {}", self.question),
            header_style,
        ));

        let mut lines = vec![BlockLine::styled(header).with_selection_range(Some(0))];

        // Collapsed: header, then a `[Copy]` row for the whole answer. Expanded: header, separator, markdown body.
        if !is_collapsed {
            lines.push(BlockLine::separator(Line::from("")));
            let body = self.content.output(ctx.width as usize);
            for mut bl in body.lines {
                bl.selection_range = Some(0);
                lines.push(bl);
            }
        } else {
            let style = Style::default().fg(theme.gray);
            lines.push(BlockLine::separator(Line::from(Span::styled(
                "[Copy]",
                style,
            ))));
        }

        BlockOutput { lines }
    }

    fn code_copy_affordances(&self, ctx: &BlockContext) -> Vec<CodeCopyAffordance> {
        // The folded row hides the answer. The button copies the source, not the one visible header.
        if ctx.mode != DisplayMode::Collapsed {
            return Vec::new();
        }
        vec![CodeCopyAffordance {
            row_offset: 1,
            label: "[Copy]".to_string(),
            body: self.content.text(),
        }]
    }

    fn accent(&self, _ctx: &BlockContext) -> Option<AccentStyle> {
        None
    }

    fn has_bullet(&self, _ctx: &BlockContext) -> bool {
        true
    }

    fn has_vpad_for(&self, _appearance: &AppearanceConfig) -> bool {
        false
    }

    fn has_raw_mode(&self) -> bool {
        false
    }

    fn is_foldable(&self) -> bool {
        true
    }

    fn is_groupable(&self) -> bool {
        true
    }

    fn default_display_mode(&self) -> DisplayMode {
        DisplayMode::Collapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrollback::types::line_plain_text;

    fn context(mode: DisplayMode) -> BlockContext {
        BlockContext {
            width: 80,
            mode,
            is_running: false,
            raw: false,
            max_lines: None,
            appearance: Default::default(),
            is_selected: false,
            cwd: None,
        }
    }

    fn plain(block: &BtwBlock, mode: DisplayMode) -> String {
        block
            .output(&context(mode))
            .lines
            .iter()
            .map(|line| line_plain_text(&line.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn folded_row_copy_button_holds_the_whole_answer() {
        let answer = "visible lead\n\nHIDDEN_ANSWER_TOKEN sits past the fold\n";
        let block = BtwBlock::new("what changed", answer);
        let folded = plain(&block, DisplayMode::Collapsed);
        assert_eq!(folded, "/btw what changed\n[Copy]");
        assert!(!folded.contains("HIDDEN_ANSWER_TOKEN"));
        let affordances = block.code_copy_affordances(&context(DisplayMode::Collapsed));
        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].row_offset, 1);
        assert_eq!(affordances[0].label, "[Copy]");
        assert_eq!(affordances[0].clipboard_text(), answer);
        let expanded = plain(&block, DisplayMode::Expanded);
        assert!(expanded.contains("HIDDEN_ANSWER_TOKEN"), "{expanded}");
        assert!(
            block
                .code_copy_affordances(&context(DisplayMode::Expanded))
                .is_empty()
        );
    }
}
