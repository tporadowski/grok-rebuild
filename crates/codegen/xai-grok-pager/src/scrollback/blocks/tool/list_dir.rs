//! ListDirToolCallBlock - lists directory contents.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::scrollback::block::BlockContent;
use crate::scrollback::types::{
    AccentStyle, BlockBackground, BlockContext, BlockLine, BlockOutput, DisplayMode, Selectable,
};
use crate::theme::Theme;

use super::{TOOL_HEADER_RANGE, collapsed_error_suffix};
use crate::appearance::AppearanceConfig;

/// Selection range for list output and error-body lines (header is 0).
const LIST_DIR_BODY_RANGE: u16 = 1;

/// Files and folders in a `list_dir` body.
/// The first unindented `- name/` bullet is the directory being listed, not a child.
/// `[N files in subtree: …]` adds those files; the summary line itself is not an item.
fn count_files_and_folders(output: &str) -> (usize, usize) {
    let mut files = 0usize;
    let mut folders = 0usize;
    let mut skipped_root = false;
    for raw in output.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(nested) = subtree_file_count(line) {
            files += nested;
            continue;
        }
        let (bullet, name) = if let Some(name) = line.strip_prefix("- ") {
            (true, name.trim())
        } else if !line.contains([' ', '\t']) {
            (false, line)
        } else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let is_dir = name.ends_with('/');
        let unindented = !raw.starts_with([' ', '\t']);
        if is_dir && bullet && unindented && !skipped_root {
            skipped_root = true;
            continue;
        }
        if is_dir {
            folders += 1;
        } else {
            files += 1;
        }
    }
    (files, folders)
}

/// `[12 files in subtree: …]` or `[1 file in subtree: …]`.
fn subtree_file_count(line: &str) -> Option<usize> {
    let rest = line.strip_prefix('[')?;
    let (num, rest) = rest.split_once(' ')?;
    if rest.starts_with("file in subtree:") || rest.starts_with("files in subtree:") {
        num.parse().ok()
    } else {
        None
    }
}

fn files_and_folders_suffix(files: usize, folders: usize) -> String {
    let file_part = match files {
        0 => None,
        1 => Some("1 file".to_string()),
        n => Some(format!("{n} files")),
    };
    let folder_part = match folders {
        0 => None,
        1 => Some("1 folder".to_string()),
        n => Some(format!("{n} folders")),
    };
    match (file_part, folder_part) {
        (None, None) => String::new(),
        (Some(part), None) | (None, Some(part)) => format!(" ({part})"),
        (Some(files), Some(folders)) => format!(" ({files}, {folders})"),
    }
}

/// List directory tool call.
#[derive(Debug, Clone)]
pub struct ListDirToolCallBlock {
    pub path: String,
    pub output: String,
    pub error: Option<String>,
    /// When the tool started running.
    pub started_at: Option<std::time::Instant>,
    /// Elapsed time in ms after completion.
    pub elapsed_ms: Option<i64>,
    /// Whether this ordinary list targets a memory v2 scope.
    pub is_memory_activity: bool,
}

impl ListDirToolCallBlock {
    /// Pre-completed blocks have no meaningful local timing; `started_at` is `None`.
    /// Timing is only set for blocks that enter a running UI state (via `set_last_running(true)` in `ScrollbackState`).
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            output: String::new(),
            error: None,
            started_at: None,
            elapsed_ms: None,
            is_memory_activity: false,
        }
    }

    pub fn with_memory_activity(mut self) -> Self {
        self.is_memory_activity = true;
        self
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = output.into();
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Set error; compute elapsed time if not already set.
    pub fn set_error(&mut self, error: Option<String>) {
        if self.elapsed_ms.is_none()
            && let Some(start) = self.started_at
        {
            self.elapsed_ms = Some(start.elapsed().as_millis() as i64);
        }
        self.error = error;
    }

    /// Finalize elapsed time from `started_at`.
    ///
    /// Idempotent: no-op if `started_at` is `None` (pre-completed block) or if `elapsed_ms` is already set (already finalized).
    pub fn finish(&mut self) {
        if self.elapsed_ms.is_some() {
            return;
        }
        if let Some(start) = self.started_at {
            self.elapsed_ms = Some(start.elapsed().as_millis() as i64);
        }
    }

    pub fn elapsed_ms(&self) -> Option<i64> {
        match self.elapsed_ms {
            Some(ms) => Some(ms),
            None => self
                .started_at
                .map(|start| start.elapsed().as_millis() as i64),
        }
    }

    pub fn set_output(&mut self, output: impl Into<String>) {
        self.output = output.into();
    }

    /// Render collapsed line: `List path` with either entry count (success)
    /// or a short error suffix (failure).
    ///
    /// When `width` is provided, the path is fish-shortened to fit alongside
    /// any suffix.
    fn collapsed_line(&self, theme: &Theme, muted: bool, width: Option<usize>) -> Line<'static> {
        let text_style = if muted {
            theme.muted()
        } else {
            theme.primary()
        };
        let bold_style = text_style.add_modifier(Modifier::BOLD);
        let path_style = if muted {
            theme.muted()
        } else {
            theme.fg(theme.path)
        };
        let error_style = if muted {
            theme.muted()
        } else {
            Style::default().fg(theme.accent_error)
        };

        let prefix = "List ";
        let error_suffix = self
            .error
            .as_ref()
            .map(|e| collapsed_error_suffix(e, 48))
            .unwrap_or_default();

        // Success path: "(N files, M folders)" when it fits. A line count treats
        // the listed directory, a subtree summary, and a notice as entries.
        let (file_count, folder_count) = if self.error.is_none() {
            count_files_and_folders(&self.output)
        } else {
            (0, 0)
        };
        let count_suffix = files_and_folders_suffix(file_count, folder_count);
        let count_fits = width.is_none_or(|w| prefix.len() + count_suffix.len() < w);
        let effective_count = if count_fits {
            count_suffix.as_str()
        } else {
            ""
        };

        let suffix_len = if !error_suffix.is_empty() {
            error_suffix.len()
        } else {
            effective_count.len()
        };
        let path_budget = width
            .map(|w| w.saturating_sub(prefix.len()).saturating_sub(suffix_len))
            .unwrap_or(usize::MAX);
        let path = crate::render::tool_paths::shorten_path(&self.path, path_budget);

        let mut spans = vec![
            Span::styled(prefix, bold_style),
            Span::styled(path, path_style),
        ];
        if !error_suffix.is_empty() {
            spans.push(Span::styled(error_suffix, error_style));
        } else if !effective_count.is_empty() {
            spans.push(Span::styled(effective_count.to_string(), theme.muted()));
        }
        Line::from(spans)
    }

    /// Header line with only the path span selectable (exclude "List " prefix).
    fn header_block_line(&self, line: Line<'static>) -> BlockLine {
        let path_end = 2.min(line.spans.len()).max(1);
        BlockLine {
            selectable: Selectable::Spans(1..path_end),
            selection_range: Some(TOOL_HEADER_RANGE),
            selection_text: Some(self.path.clone()),
            content: line,
            ..Default::default()
        }
    }
}

impl BlockContent for ListDirToolCallBlock {
    fn output(&self, ctx: &BlockContext) -> BlockOutput {
        let theme = Theme::current();
        let muted_collapsed =
            ctx.mute_when_collapsed(ctx.appearance.scrollback.blocks.tool.muted_collapsed);
        let terminal_bg = ctx.appearance.scrollback.blocks.list_dir.terminal_bg;

        match ctx.mode {
            DisplayMode::Collapsed => BlockOutput {
                lines: vec![self.header_block_line(self.collapsed_line(
                    &theme,
                    muted_collapsed,
                    Some(ctx.content_width()),
                ))],
            },
            DisplayMode::Truncated | DisplayMode::Expanded => {
                let mut lines: Vec<BlockLine> =
                    vec![self.header_block_line(self.collapsed_line(&theme, false, None))];

                if let Some(err) = &self.error {
                    // Blank gap is decoration. Error body shares the header
                    // range so text drag can span path + failure details.
                    lines.push(BlockLine::separator(Line::from("")));
                    let error_style = Style::default().fg(theme.accent_error);
                    for line in err.lines() {
                        lines.push(
                            BlockLine::styled(Line::from(Span::styled(
                                line.to_string(),
                                error_style,
                            )))
                            .with_selection_range(Some(TOOL_HEADER_RANGE)),
                        );
                    }
                } else if !self.output.is_empty() {
                    lines.push(BlockLine::separator(Line::from("")));

                    for rl in crate::render::terminal_output::render_terminal_lines(
                        &self.output,
                        theme.primary(),
                    ) {
                        // Indent output by 2 spaces
                        let mut spans = vec![Span::styled("  ".to_string(), theme.primary())];
                        spans.extend(rl.line.spans);
                        let mut block_line: BlockLine = Line::from(spans).into();
                        block_line = block_line.with_selection_range(Some(LIST_DIR_BODY_RANGE));
                        if terminal_bg {
                            block_line = block_line.with_panel_background(theme.bg_dark);
                        }
                        lines.push(block_line);
                    }
                }

                BlockOutput { lines }
            }
        }
    }

    fn accent(&self, _ctx: &BlockContext) -> Option<AccentStyle> {
        None // ListDir blocks never have an accent line
    }

    fn bullet(&self, _ctx: &BlockContext) -> Option<AccentStyle> {
        if self.error.is_some() {
            let theme = Theme::current();
            Some(AccentStyle::static_color(theme.accent_error))
        } else {
            None
        }
    }

    fn has_vpad_for(&self, _appearance: &AppearanceConfig) -> bool {
        false
    }

    fn background(&self, _ctx: &BlockContext) -> BlockBackground {
        BlockBackground::None
    }

    fn has_raw_mode(&self) -> bool {
        false
    }

    fn is_foldable(&self) -> bool {
        // Listing content, or a failure message the user can expand to read
        // in full (collapsed already shows a short error suffix).
        !self.output.is_empty() || self.error.is_some()
    }

    fn default_display_mode(&self) -> DisplayMode {
        // A failed listing opens so the full reason is on screen. A short header
        // suffix drops every line after the first.
        if self.error.is_some() {
            DisplayMode::Expanded
        } else {
            DisplayMode::Collapsed
        }
    }

    fn next_fold_mode(&self, current: DisplayMode, _is_running: bool) -> DisplayMode {
        match current {
            DisplayMode::Collapsed => DisplayMode::Expanded,
            _ => DisplayMode::Collapsed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrollback::types::{BlockContext, DisplayMode};

    fn make_ctx() -> BlockContext {
        BlockContext {
            width: 80,
            mode: DisplayMode::Collapsed,
            is_running: false,
            raw: false,
            max_lines: None,
            appearance: Default::default(),
            is_selected: false,
            cwd: None,
        }
    }

    fn header_text(block: &ListDirToolCallBlock, ctx: &BlockContext) -> String {
        let output = block.output(ctx);
        let Some(line) = output.lines.first() else {
            panic!("expected a header line");
        };
        line.content
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect()
    }

    #[test]
    fn collapsed_header_shows_entry_count() {
        let block = ListDirToolCallBlock::new("src").with_output("a.rs\nb.rs\nsub/\n");
        assert_eq!(header_text(&block, &make_ctx()), "List src (2 files, 1 folder)");

        let single = ListDirToolCallBlock::new("src").with_output("lonely.rs\n");
        assert_eq!(header_text(&single, &make_ctx()), "List src (1 file)");
    }

    #[test]
    fn collapsed_header_counts_files_and_folders() {
        let plain = ListDirToolCallBlock::new("src").with_output("a.rs\nb.rs\nsub/\n");
        assert_eq!(
            header_text(&plain, &make_ctx()),
            "List src (2 files, 1 folder)"
        );

        let single = ListDirToolCallBlock::new("src").with_output("lonely.rs\n");
        assert_eq!(header_text(&single, &make_ctx()), "List src (1 file)");

        // The root bullet is the listed directory. A collapsed subtree's file
        // count is the number in the summary, not one extra entry for that line.
        let listed = "\
- src/
  - a.rs
  - sub/
    [2 files in subtree: 2 *.rs]
";
        let nested = ListDirToolCallBlock::new("src").with_output(listed);
        assert_eq!(
            header_text(&nested, &make_ctx()),
            "List src (3 files, 1 folder)"
        );

        let only_folder = ListDirToolCallBlock::new("src").with_output("docs/\n");
        assert_eq!(header_text(&only_folder, &make_ctx()), "List src (1 folder)");
    }

    #[test]
    fn collapsed_header_omits_count_when_empty() {
        let empty = ListDirToolCallBlock::new("src");
        assert_eq!(header_text(&empty, &make_ctx()), "List src");
    }

    #[test]
    fn collapsed_failure_shows_path_and_error_reason() {
        let block = ListDirToolCallBlock::new(".claude/skills")
            .with_error("Error: Directory does not exist: .claude/skills");
        let text = header_text(&block, &make_ctx());
        assert_eq!(
            text, "List .claude/skills — Directory does not exist",
            "path once in tool target; reason without path suffix, got '{text}'"
        );
    }

    #[test]
    fn collapsed_failure_omits_entry_count() {
        // Failures show an error suffix, never a success entry count.
        let failed = ListDirToolCallBlock::new("gone")
            .with_output("stale\n")
            .with_error("no such directory");
        let text = header_text(&failed, &make_ctx());
        assert_eq!(text, "List gone — no such directory");
        assert!(!text.contains("entr"));
    }

    #[test]
    fn foldable_when_error_even_without_output() {
        let block = ListDirToolCallBlock::new("missing").with_error("Permission denied: missing");
        assert!(block.is_foldable());
    }

    #[test]
    fn not_foldable_when_empty_success() {
        let block = ListDirToolCallBlock::new("empty");
        assert!(!block.is_foldable());
    }

    #[test]
    fn failed_directory_listing_expands_to_show_why() {
        let reason = "Error: Directory does not exist: /proj/.claude/skills\n\
The parent folder is missing, so there is nothing to list.";
        let failed = ListDirToolCallBlock::new(".claude/skills").with_error(reason);
        assert_eq!(failed.default_display_mode(), DisplayMode::Expanded);
        let ok = ListDirToolCallBlock::new("src").with_output("a.rs\n");
        assert_eq!(ok.default_display_mode(), DisplayMode::Collapsed);

        let mut state = crate::scrollback::ScrollbackState::new();
        let running = state.push_block(crate::scrollback::RenderBlock::ToolCall(
            super::super::ToolCallBlock::ListDir(ListDirToolCallBlock::new(".claude/skills")),
        ));
        assert_eq!(
            state.get_by_id(running).unwrap().display_mode,
            DisplayMode::Collapsed
        );
        assert!(state.replace_tool_block(
            running,
            crate::scrollback::RenderBlock::ToolCall(super::super::ToolCallBlock::ListDir(
                failed.clone()
            )),
            None,
        ));
        let entry = state.get_by_id_mut(running).unwrap();
        assert_eq!(entry.display_mode, DisplayMode::Expanded);
        let shown = entry.output(80, &crate::appearance::AppearanceConfig::default(), None);
        let text: String = shown
            .lines
            .iter()
            .map(|line| {
                line.content
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("The parent folder is missing, so there is nothing to list."),
            "the opened listing keeps the reason the header cuts: {text}"
        );

        let inserted = state.push_block(crate::scrollback::RenderBlock::ToolCall(
            super::super::ToolCallBlock::ListDir(failed),
        ));
        assert_eq!(
            state.get_by_id(inserted).unwrap().display_mode,
            DisplayMode::Expanded
        );
    }

    #[test]
    fn expanded_failure_shows_full_error_body() {
        let block = ListDirToolCallBlock::new("secret")
            .with_error("Permission denied: secret\n(no listing available)");
        let mut ctx = make_ctx();
        ctx.mode = DisplayMode::Expanded;
        let all_text: String = block
            .output(&ctx)
            .lines
            .iter()
            .map(|l| {
                l.content
                    .spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            all_text.contains("Permission denied: secret"),
            "got '{all_text}'"
        );
        assert!(
            all_text.contains("(no listing available)"),
            "got '{all_text}'"
        );
    }

    #[test]
    fn expanded_failure_error_lines_are_selectable() {
        let block = ListDirToolCallBlock::new("secret")
            .with_error("Permission denied: secret\n(no listing available)");
        let mut ctx = make_ctx();
        ctx.mode = DisplayMode::Expanded;
        let output = block.output(&ctx);
        let body: Vec<_> = output
            .lines
            .iter()
            .filter(|l| l.selection_range == Some(TOOL_HEADER_RANGE))
            .filter(|l| !matches!(l.selectable, Selectable::Spans(_)))
            .collect();
        assert_eq!(body.len(), 2, "both error lines should be selectable");
        assert!(
            body.iter()
                .all(|l| !matches!(l.selectable, Selectable::None))
        );
        assert_eq!(
            output.lines[0].selection_range,
            Some(TOOL_HEADER_RANGE),
            "error body must share the header range"
        );
    }
}
