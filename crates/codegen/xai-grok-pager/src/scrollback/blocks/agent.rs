use crate::scrollback::block::BlockContent;
use crate::scrollback::types::{AccentStyle, BlockContext, BlockOutput};

use super::markdown_content::MarkdownContent;
use super::mermaid_content::{self, MermaidContent};
use crate::appearance::AppearanceConfig;

/// Block displaying an agent message with streaming markdown support. When text arrives in chunks, call
/// `push_chunk()` to append without re-rendering everything.
#[derive(Debug, Clone)]
pub struct AgentMessageBlock {
    content: MarkdownContent,
    /// Cached image references extracted from the markdown source.
    image_refs: Vec<crate::prompt_images::ScrollbackImageRef>,
    /// Cached video references extracted from the markdown source.
    video_refs: Vec<crate::prompt_images::ScrollbackVideoRef>,
    /// Detected ` ```mermaid ` diagrams and render skeleton, populated at construction/finish (never per streaming chunk) like the media refs.
    mermaid: MermaidContent,
}

impl AgentMessageBlock {
    /// Create a new agent message block with complete text.
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let image_refs = crate::prompt_images::extract_image_refs(&text);
        let video_refs = crate::prompt_images::extract_video_refs(&text);
        let content = MarkdownContent::new(text);
        let mermaid = content.mermaid_content();
        Self {
            content,
            image_refs,
            video_refs,
            mermaid,
        }
    }

    /// Create an empty block for streaming.
    pub fn streaming() -> Self {
        Self {
            content: MarkdownContent::streaming(),
            image_refs: Vec::new(),
            video_refs: Vec::new(),
            mermaid: MermaidContent::default(),
        }
    }

    /// Push a streaming chunk of markdown text.
    pub fn push_chunk(&mut self, chunk: &str) {
        self.content.push_chunk(chunk);
    }

    /// Push a chunk without rendering immediately.
    pub fn push_chunk_deferred(&mut self, chunk: &str) {
        self.content.push_chunk_deferred(chunk);
    }

    /// Get the source markdown text.
    pub fn text(&self) -> String {
        self.content.text()
    }

    /// Whether the source markdown is empty (zero-alloc, unlike `text()`).
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Finish streaming and do a full re-render for safety.
    pub fn finish(&mut self) {
        self.content.finish();
        let text = self.content.text();
        self.image_refs = crate::prompt_images::extract_image_refs(&text);
        self.video_refs = crate::prompt_images::extract_video_refs(&text);
        // Detection runs once the render is final, after the renderer freezes, never per streaming chunk
        self.mermaid = self.content.mermaid_content();
    }

    /// The detected Mermaid diagrams for this message (empty until finished or constructed from complete text).
    pub fn mermaid(&self) -> &MermaidContent {
        &self.mermaid
    }

    /// Set the raw mode, re-rendering if it changed.
    pub fn set_raw_mode(&mut self, raw: bool) {
        self.content.set_raw_mode(raw);
    }

    /// Access the underlying markdown content (for viewer item building).
    pub fn content(&self) -> &MarkdownContent {
        &self.content
    }

    /// Mutable access to the underlying markdown content.
    pub fn content_mut(&mut self) -> &mut MarkdownContent {
        &mut self.content
    }

    /// Get copyable text for this block.
    /// When `raw` is true, returns the raw markdown source.
    /// When `raw` is false, returns the rendered text (styles stripped).
    pub fn copy_text(&self, raw: bool) -> String {
        if raw {
            self.content.text()
        } else {
            self.content.rendered_plain_text()
        }
    }
}

impl AgentMessageBlock {
    /// Resolve the diagram display mode from the user setting without building `output()`; cheap enough to gate the per-frame affordance path.
    fn mermaid_display_mode(&self) -> mermaid_content::MermaidDisplay {
        // Minimal mode commits static text with no draw loop to paint the clickable affordance row, so suppress it there
        // The diagram art still renders; its source stays natively selectable
        // The inline-overlay force-off flag is set iff minimal
        mermaid_content::mermaid_display_static(
            crate::appearance::cache::load_render_mermaid(),
            crate::terminal::image::scrollback_inline_overlay_forced_off(),
        )
    }

    /// Markdown body, then Mermaid rows, then copy-button rows under tables and code blocks.
    /// Inserts that land above a diagram shift that diagram's row.
    fn compose(
        &self,
        ctx: &BlockContext,
    ) -> (
        BlockOutput,
        Vec<mermaid_content::DiagramAffordance>,
        Vec<crate::scrollback::table_copy::TableCopyAffordance>,
        Vec<crate::scrollback::code_copy::CodeCopyAffordance>,
    ) {
        let mut out = self.content.output(ctx.width as usize);
        let mut diagrams = Vec::new();
        let show_diagrams = !ctx.raw
            && !self.mermaid.is_empty()
            && self.mermaid_display_mode() == mermaid_content::MermaidDisplay::Affordances;
        if show_diagrams {
            let ranges = self.content.mermaid_block_ranges();
            diagrams = mermaid_content::apply_affordance_rows(&mut out, &ranges, |idx| {
                self.mermaid.source(idx).unwrap_or_default().to_string()
            });
        }
        let tables = if ctx.raw || self.content.is_raw() {
            Vec::new()
        } else {
            let metas = self.content.table_copy_metas();
            let tables = crate::scrollback::table_copy::apply_table_copy_rows(&mut out, &metas);
            shift_diagrams_below(
                &mut diagrams,
                tables.iter().map(|table| table.row_offset),
            );
            tables
        };
        let codes = if ctx.raw || self.content.is_raw() || !self.content.is_finished() {
            Vec::new()
        } else {
            let spans = self.content.code_blocks();
            let codes =
                crate::scrollback::code_copy::apply_code_copy_rows(&mut out, &spans);
            shift_diagrams_below(&mut diagrams, codes.iter().map(|code| code.row_offset));
            codes
        };
        (out, diagrams, tables, codes)
    }
}

/// A button row inserted above a diagram pushes that diagram down by one row.
fn shift_diagrams_below(
    diagrams: &mut [mermaid_content::DiagramAffordance],
    final_offsets: impl Iterator<Item = u16>,
) {
    let offsets: Vec<u16> = final_offsets.collect();
    for diagram in diagrams {
        let mut shift = 0u16;
        for (index, offset) in offsets.iter().enumerate() {
            let original_at = (*offset as usize).saturating_sub(index);
            if original_at <= diagram.row_offset as usize {
                shift = shift.saturating_add(1);
            }
        }
        diagram.row_offset = diagram.row_offset.saturating_add(shift);
    }
}

impl BlockContent for AgentMessageBlock {
    fn output(&self, ctx: &BlockContext) -> BlockOutput {
        self.compose(ctx).0
    }

    fn diagram_affordances(&self, ctx: &BlockContext) -> Vec<mermaid_content::DiagramAffordance> {
        if ctx.raw
            || self.mermaid.is_empty()
            || self.mermaid_display_mode() != mermaid_content::MermaidDisplay::Affordances
        {
            return Vec::new();
        }
        self.compose(ctx).1
    }

    fn table_copy_affordances(
        &self,
        ctx: &BlockContext,
    ) -> Vec<crate::scrollback::table_copy::TableCopyAffordance> {
        if ctx.raw || self.content.is_raw() {
            return Vec::new();
        }
        self.compose(ctx).2
    }

    fn code_copy_affordances(
        &self,
        ctx: &BlockContext,
    ) -> Vec<crate::scrollback::code_copy::CodeCopyAffordance> {
        if ctx.raw || self.content.is_raw() || !self.content.is_finished() {
            return Vec::new();
        }
        self.compose(ctx).3
    }

    fn estimate_extra_rows(&self) -> u16 {
        // Diagram treatment rows and the copy-button rows are not in the source-text estimate.
        if self.content.is_raw() || !self.content.is_finished() {
            return 0;
        }
        let tables = self.content.table_count() as u16;
        let diagrams = if self.mermaid.is_empty()
            || self.mermaid_display_mode() == mermaid_content::MermaidDisplay::SourceOnly
        {
            0
        } else {
            self.mermaid.len() as u16
        };
        let codes = self
            .content
            .code_blocks()
            .iter()
            .filter(|span| {
                !span.output_line_range.is_empty()
                    && !crate::scrollback::code_copy::is_mermaid_fence(&span.info)
            })
            .count() as u16;
        diagrams.saturating_add(tables).saturating_add(codes)
    }

    fn accent(&self, _ctx: &BlockContext) -> Option<AccentStyle> {
        None
    }

    fn has_vpad_for(&self, _appearance: &AppearanceConfig) -> bool {
        false
    }

    fn has_raw_mode(&self) -> bool {
        true
    }

    fn is_foldable(&self) -> bool {
        false
    }

    fn image_references(&self) -> &[crate::prompt_images::ScrollbackImageRef] {
        &self.image_refs
    }

    fn video_references(&self) -> &[crate::prompt_images::ScrollbackVideoRef] {
        &self.video_refs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::{AppearanceConfig, RenderMermaid};
    use crate::scrollback::types::Selectable;

    fn ctx(width: u16, raw: bool) -> BlockContext {
        BlockContext {
            mode: crate::scrollback::DisplayMode::Expanded,
            is_running: false,
            width,
            raw,
            max_lines: None,
            appearance: AppearanceConfig::default(),
            is_selected: false,
            cwd: None,
        }
    }

    #[test]
    fn agent_markdown_body_uses_single_logical_range() {
        let block = AgentMessageBlock::new("hello world this should wrap across lines");
        let out = block.output(&ctx(10, false));
        assert!(out.lines.len() > 1);
        assert!(out.lines.iter().all(|line| line.selection_range == Some(0)));
        assert!(
            out.lines
                .iter()
                .all(|line| !matches!(line.selectable, Selectable::None))
        );
    }

    #[test]
    fn agent_message_shows_table_copy_buttons_under_the_table() {
        use crate::scrollback::table_copy::TableCopyFormat;

        let block = AgentMessageBlock::new(
            "before\n\n| Name | Note |\n| --- | --- |\n| Ada | a, b |\n\nafter\n",
        );
        let pretty = ctx(80, false);
        let raw_ctx = ctx(80, true);
        let out = block.output(&pretty);
        let affordances = block.table_copy_affordances(&pretty);
        assert_eq!(affordances.len(), 1, "one table, one button row");
        let row = affordances[0].row_offset as usize;
        let label = out
            .lines
            .get(row)
            .map(|line| {
                line.content
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .unwrap_or_default();
        assert!(
            label.contains("[TSV]") && label.contains("[CSV]") && label.contains("[Markdown]"),
            "buttons sit on the row under the table: {label}"
        );
        assert!(
            row > 0
                && out.lines.get(row - 1).is_some_and(|line| {
                    let text: String = line
                        .content
                        .spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect();
                    !text.contains("[TSV]")
                }),
            "the buttons are underneath the table, not inside it"
        );
        assert_eq!(affordances[0].n_cols, 2);
        assert_eq!(
            affordances[0].cells,
            ["Name", "Note", "Ada", "a, b"]
                .map(str::to_string)
                .to_vec()
        );
        assert!(matches!(
            out.lines[row].selectable,
            Selectable::None
        ));
        assert_eq!(
            affordances[0].clipboard_text(TableCopyFormat::Tsv),
            "Name\tNote\nAda\ta, b"
        );
        assert_eq!(
            affordances[0].clipboard_text(TableCopyFormat::Csv),
            "Name,Note\nAda,\"a, b\""
        );
        assert_eq!(
            affordances[0].clipboard_text(TableCopyFormat::Markdown),
            "| Name | Note |\n| --- | --- |\n| Ada | a, b |"
        );
        assert!(!block.copy_text(false).contains("[TSV]"));
        assert_eq!(block.estimate_extra_rows(), 1);
        let texts: Vec<String> = out
            .lines
            .iter()
            .map(|line| {
                line.content
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();
        let ada = texts.iter().position(|text| text.contains("Ada")).unwrap();
        let after = texts.iter().position(|text| text.contains("after")).unwrap();
        assert!(
            ada < row && row < after,
            "the button row sits between the table and the text after it"
        );
        let raw = block.output(&raw_ctx);
        assert!(
            raw.lines.iter().all(|line| {
                let text: String = line
                    .content
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect();
                !text.contains("[TSV]")
            }),
            "raw mode shows the source table, not the copy buttons"
        );
        assert!(
            AgentMessageBlock::new("no table here")
                .table_copy_affordances(&pretty)
                .is_empty()
        );
    }

    #[test]
    fn agent_copy_text_preserves_raw_semantics() {
        let block = AgentMessageBlock::new("**bold** text");
        assert_eq!(block.copy_text(true), "**bold** text");
        assert_eq!(
            block.copy_text(false),
            block.content().rendered_plain_text()
        );
    }

    fn line_text(line: &crate::scrollback::types::BlockLine) -> String {
        line.content
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn finished_reply_puts_a_labeled_copy_button_under_each_code_block() {
        let block = AgentMessageBlock::new(
            "intro\n\n```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\n```10:12:src/lib.rs\nlet x = 1;\n```\n\n```\nplain\n```\n\noutro\n",
        );
        let pretty = ctx(60, false);
        let out = block.output(&pretty);
        let affordances = block.code_copy_affordances(&pretty);
        assert_eq!(affordances.len(), 3);
        assert_eq!(affordances[0].label, "[Copy] rust");
        assert!(affordances[0].clipboard_text().contains("fn main()"));
        assert_eq!(affordances[1].label, "[Copy] lib.rs");
        assert!(affordances[1].clipboard_text().contains("let x = 1;"));
        assert_eq!(affordances[2].label, "[Copy]");
        assert!(affordances[2].clipboard_text().contains("plain"));
        let texts: Vec<String> = out.lines.iter().map(line_text).collect();
        let rust_row = affordances[0].row_offset as usize;
        let file_row = affordances[1].row_offset as usize;
        let plain_row = affordances[2].row_offset as usize;
        assert_eq!(texts[rust_row], "[Copy] rust");
        assert_eq!(texts[file_row], "[Copy] lib.rs");
        assert_eq!(texts[plain_row], "[Copy]");
        assert!(
            texts[..rust_row].iter().any(|text| text.contains("fn main()")),
            "the rust button sits under its code: {texts:?}"
        );
        assert!(
            texts[rust_row + 1..]
                .iter()
                .any(|text| text.contains("let x = 1;")),
            "the next block stays below the rust button"
        );
        assert!(matches!(
            out.lines[rust_row].selectable,
            Selectable::None
        ));
        assert!(!block.copy_text(false).contains("[Copy]"));
        assert_eq!(block.estimate_extra_rows(), 3);
        let raw = block.output(&ctx(60, true));
        assert!(
            raw.lines.iter().all(|line| !line_text(line).contains("[Copy]")),
            "raw mode shows the fence, not the button"
        );

        let mut streaming = AgentMessageBlock::streaming();
        streaming.push_chunk("```rust\nlet y = 2;\n```\n");
        assert!(
            !streaming.content().is_finished(),
            "a stream is not a finished reply"
        );
        assert!(
            streaming
                .output(&pretty)
                .lines
                .iter()
                .all(|line| !line_text(line).contains("[Copy]")),
            "the button waits until the reply finishes"
        );
        streaming.finish();
        let finished = streaming.code_copy_affordances(&pretty);
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].label, "[Copy] rust");
        assert!(finished[0].clipboard_text().contains("let y = 2;"));
    }

    #[test]
    fn mermaid_detected_at_construction() {
        let block = AgentMessageBlock::new("```mermaid\nflowchart TD\n  A --> B\n```\n");
        assert_eq!(block.mermaid().len(), 1);
        assert_eq!(block.mermaid().source(0), Some("flowchart TD\n  A --> B\n"));
    }

    #[test]
    fn mermaid_not_detected_during_streaming_until_finish() {
        let mut block = AgentMessageBlock::streaming();
        block.push_chunk("```mermaid\nflowchart TD\n");
        // Fence still open mid-stream, so no detection
        assert!(block.mermaid().is_empty());
        block.push_chunk("A --> B\n```\n");
        assert!(
            block.mermaid().is_empty(),
            "detection runs at finish(), not per chunk"
        );
        block.finish();
        assert_eq!(block.mermaid().len(), 1);
    }

    const MERMAID_MD: &str = "```mermaid\nflowchart TD\n  A --> B\n```\n";

    #[test]
    fn mermaid_treatment_row_shown_in_auto_not_off_not_raw() {
        let non_selectable = |o: &BlockOutput| {
            o.lines
                .iter()
                .filter(|l| matches!(l.selectable, Selectable::None))
                .count()
        };
        // off: plain code block, no extra row
        crate::appearance::cache::set_render_mermaid(RenderMermaid::Off);
        let off = AgentMessageBlock::new(MERMAID_MD).output(&ctx(40, false));
        assert_eq!(non_selectable(&off), 0, "off mode must not add a row");

        // auto: exactly one extra non-selectable row beneath the diagram (the affordance row)
        // The row is blank in `output()`; the draw loop paints its `◇ mermaid [Open Image] [Copy Image Path] [Copy Source]` buttons
        crate::appearance::cache::set_render_mermaid(RenderMermaid::Auto);
        let auto = AgentMessageBlock::new(MERMAID_MD).output(&ctx(40, false));
        assert_eq!(
            auto.lines.len(),
            off.lines.len() + 1,
            "auto mode adds one treatment row"
        );
        assert_eq!(non_selectable(&auto), 1);

        // raw: verbatim source, no extra row even in auto
        crate::appearance::cache::set_render_mermaid(RenderMermaid::Auto);
        let raw = AgentMessageBlock::new(MERMAID_MD).output(&ctx(40, true));
        assert_eq!(non_selectable(&raw), 0, "raw mode shows the fence verbatim");
    }

    #[test]
    fn mermaid_treatment_row_preserves_hyperlink_line_mapping() {
        // The inserted treatment row (caption or affordance) is a joiner-continuation line, so it must NOT add a logical (pre-wrap) line
        // Otherwise the hyperlink overlay walk desyncs for the paragraph after the diagram
        let md = "before\n\n```mermaid\nA-->B\n```\n\n[link](https://example.com) trailing\n";
        crate::appearance::cache::set_render_mermaid(RenderMermaid::Off);
        let off = AgentMessageBlock::new(md).output(&ctx(60, false));
        crate::appearance::cache::set_render_mermaid(RenderMermaid::Auto);
        let block = AgentMessageBlock::new(md);
        let auto = block.output(&ctx(60, false));

        let logical = |o: &BlockOutput| o.lines.iter().filter(|l| l.joiner.is_none()).count();
        assert_eq!(
            logical(&auto),
            logical(&off),
            "treatment row must not introduce a new logical line",
        );

        // The renderer's hyperlinks are pre-wrap and unchanged by the inserted row (it lives in the BlockOutput, not the renderer)
        // Walk the output's joiners to recover each row's pre-wrap index
        // The link's pre-wrap line must still map to its row; the inserted row did not shift it
        let link_line = block
            .content()
            .with_hyperlinks(|hs| hs.iter().map(|h| h.line_index).min())
            .expect("the trailing link must be detected");
        let mut prewrap = 0usize;
        let mut mapped_text = String::new();
        for (row, line) in auto.lines.iter().enumerate() {
            if row > 0 && line.joiner.is_none() {
                prewrap += 1;
            }
            if prewrap == link_line {
                mapped_text = line
                    .content
                    .spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect();
                break;
            }
        }
        assert!(
            mapped_text.contains("example.com"),
            "link pre-wrap line {link_line} must still map to the link row, got {mapped_text:?}",
        );
    }

    // -- diagram affordance rows ---------------------------------------------

    mod affordances {
        use super::*;

        fn shown_text(out: &BlockOutput) -> String {
            out.lines
                .iter()
                .flat_map(|l| l.content.spans.iter())
                .map(|s| s.content.as_ref())
                .collect()
        }

        /// A detected diagram keeps its source code block on screen and exposes a single affordance row.
        /// The row carries the diagram source, the data every lazy `[Open]`/`[Copy path]`/`[Copy source]` button acts on.
        /// Rendering is lazy, so no path/state is tracked on the row.
        #[test]
        fn diagram_exposes_affordance_carrying_source_and_keeps_source_block() {
            crate::appearance::cache::set_render_mermaid(RenderMermaid::On);
            let block = AgentMessageBlock::new("intro\n\n```mermaid\nA-->B\n```\n\nbye\n");

            let affs = block.diagram_affordances(&ctx(60, false));
            assert_eq!(affs.len(), 1, "one diagram → one affordance row");
            let Some(aff) = affs.first() else {
                panic!("expected one affordance: {affs:?}");
            };
            assert_eq!(aff.source, "A-->B\n");

            // The diagram is shown as its source code block (never an image)
            // The affordance row sits at its reported (non-selectable) offset
            let out = block.output(&ctx(60, false));
            assert!(
                shown_text(&out).contains("A-->B"),
                "the source code block stays on screen",
            );
            assert!(matches!(
                out.lines
                    .get(aff.row_offset as usize)
                    .map(|l| &l.selectable),
                Some(&Selectable::None)
            ));
        }

        #[test]
        fn raw_mode_suppresses_affordances_and_shows_source() {
            crate::appearance::cache::set_render_mermaid(RenderMermaid::On);
            let block = AgentMessageBlock::new("```mermaid\nA-->B\n```\n");

            assert!(
                block.diagram_affordances(&ctx(60, true)).is_empty(),
                "raw mode suppresses affordances",
            );
            assert!(shown_text(&block.output(&ctx(60, true))).contains("A-->B"));

            // Toggling back to pretty restores the affordance row.
            assert_eq!(block.diagram_affordances(&ctx(60, false)).len(), 1);
        }

        #[test]
        fn off_setting_shows_source_with_no_affordances() {
            crate::appearance::cache::set_render_mermaid(RenderMermaid::Off);
            let block = AgentMessageBlock::new("```mermaid\nA-->B\n```\n");
            assert!(block.diagram_affordances(&ctx(60, false)).is_empty());
            assert!(shown_text(&block.output(&ctx(60, false))).contains("A-->B"));
        }

        #[test]
        fn copy_over_diagram_yields_fence_body() {
            use crate::scrollback::block::RenderBlock;
            crate::appearance::cache::set_render_mermaid(RenderMermaid::On);
            // Drive the real whole-block copy path (`copy_visible_text_in_state`, then `plain_text_from_output`)
            // Re-implementing the selectable filter would drift from production
            // The source code block is selectable; the blank affordance row is excluded
            let block = RenderBlock::AgentMessage(AgentMessageBlock::new(
                "```mermaid\nA-->B\nC-->D\n```\n",
            ));
            let copied = block
                .copy_visible_text_in_state(&ctx(60, false))
                .expect("the source code block yields selectable copy text");
            assert!(copied.contains("A-->B"), "copy yields source: {copied:?}");
            assert!(copied.contains("C-->D"), "copy yields source: {copied:?}");
        }

        /// With two diagrams, each affordance row anchors at its OWN (non-selectable) row in the final output, in document order.
        /// Each row carries that diagram's own source.
        #[test]
        fn two_diagrams_each_anchor_at_their_own_row() {
            crate::appearance::cache::set_render_mermaid(RenderMermaid::On);
            let md = "intro line\n\n```mermaid\nAAA-->BBB\n```\n\nmid line\n\n```mermaid\nCCC-->DDD\n```\n\nbye line\n";
            let block = AgentMessageBlock::new(md);
            assert_eq!(block.mermaid().len(), 2, "two diagrams");

            let out = block.output(&ctx(60, false));
            let affs = block.diagram_affordances(&ctx(60, false));
            assert_eq!(affs.len(), 2);
            let [a0, a1] = affs.as_slice() else {
                panic!("expected two affordances: {affs:?}");
            };
            assert!(
                a0.row_offset < a1.row_offset,
                "diagram order preserved: {} < {}",
                a0.row_offset,
                a1.row_offset,
            );
            assert_eq!(a0.source, "AAA-->BBB\n");
            assert_eq!(a1.source, "CCC-->DDD\n");
            for aff in &affs {
                assert!(matches!(
                    out.lines
                        .get(aff.row_offset as usize)
                        .map(|l| &l.selectable),
                    Some(&Selectable::None)
                ));
            }
            // Both diagrams' sources remain visible as code blocks.
            assert!(shown_text(&out).contains("AAA-->BBB"));
            assert!(shown_text(&out).contains("CCC-->DDD"));
        }
    }
}
