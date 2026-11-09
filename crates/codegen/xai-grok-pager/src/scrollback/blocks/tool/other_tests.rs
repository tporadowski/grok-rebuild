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

/// Minimal valid 1x1 PNG.
const PNG_BYTES: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
    0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
    0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
    0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn rendered(block: &OtherToolCallBlock, mode: DisplayMode) -> String {
    block
        .output(&context(mode))
        .lines
        .iter()
        .map(|line| line_plain_text(&line.content))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn generated_media_path_fits_the_row_and_keeps_the_file_name() {
    use unicode_width::UnicodeWidthStr;

    let dir = tempfile::tempdir().expect("tempdir");
    let wide = dir.path().join("文文文文文文");
    std::fs::create_dir(&wide).expect("wide dir");
    // Backslashes are part of the name on Unix, and they are separators in the displayed path.
    let image_path = wide.join("生成\\图片.png");
    let video_path = wide.join("生成\\帧😀.mp4");
    std::fs::write(&image_path, PNG_BYTES).expect("png");
    std::fs::write(&video_path, b"clip").expect("mp4");

    let image = OtherToolCallBlock::new("image_gen", "saved").with_media_ref(&image_path, false);
    let video = OtherToolCallBlock::new("video_gen", "saved").with_media_ref(&video_path, true);
    assert!(image.media_ref_path().is_some(), "image file is a media ref");
    assert!(video.media_ref_path().is_some(), "video file is a media ref");

    let stored = image.media_ref_path().expect("image path").display().to_string();
    let mut ctx = context(DisplayMode::Expanded);
    let bullet = (ctx.width as usize).saturating_sub(ctx.content_width());
    let chars = stored.chars().count();
    assert!(
        stored.width() > chars,
        "the fixture must be wider than its char count: {stored}"
    );
    ctx.width = (chars + bullet) as u16;
    let budget = ctx.content_width();
    assert_eq!(budget, chars, "char-count truncation would leave this path alone");

    let image_line = rendered_in(&image, &ctx)
        .into_iter()
        .nth(1)
        .expect("image filepath line");
    assert!(
        image_line.width() <= budget,
        "image path must fit {budget} columns, got {} ({image_line})",
        image_line.width()
    );
    assert!(
        image_line.ends_with("图片.png"),
        "the image file name stays whole: {image_line}"
    );

    let video_line = rendered_in(&video, &ctx)
        .into_iter()
        .nth(1)
        .expect("video filepath line");
    assert!(
        video_line.width() <= budget,
        "video path must fit {budget} columns, got {} ({video_line})",
        video_line.width()
    );
    assert!(
        video_line.ends_with("帧😀.mp4"),
        "the video file name stays whole: {video_line}"
    );

    ctx.width = (8 + bullet) as u16;
    let narrow = ctx.content_width();
    let narrow_line = rendered_in(&image, &ctx)
        .into_iter()
        .nth(1)
        .expect("narrow filepath line");
    assert!(
        narrow_line.width() <= narrow,
        "a file name wider than the row still fits {narrow}: {narrow_line}"
    );
    assert!(
        narrow_line.contains(".png"),
        "the extension of a too-wide file name stays: {narrow_line}"
    );
}

#[test]
fn failed_call_expands_to_its_error() {
    let block = OtherToolCallBlock::new("computer_use__computer_apps", "")
        .with_error("the helper did not answer within 15s");
    assert!(block.is_foldable(), "a failure has a body to show");
    let expanded = rendered(&block, DisplayMode::Expanded);
    assert!(
        expanded.contains("the helper did not answer within 15s"),
        "{expanded}"
    );
    assert!(!rendered(&block, DisplayMode::Collapsed).contains("did not answer"));
}

#[test]
fn success_without_output_stays_unfoldable() {
    assert!(!OtherToolCallBlock::new("tool", "done").is_foldable());
}

/// Two Path A answers. The trailing sentence is the suffix the parser strips.
fn answered_questions_output() -> OtherToolCallBlock {
    OtherToolCallBlock::new("AskUserQuestion", "").with_output(
        "User has answered your questions: \
         \"Which runtime?\"=\"tokio\", \"Which port?\"=\"8080\". \
         You can now continue with the user's answers in mind.",
    )
}

fn rendered_in(block: &OtherToolCallBlock, ctx: &BlockContext) -> Vec<String> {
    block
        .output(ctx)
        .lines
        .iter()
        .map(|line| line_plain_text(&line.content))
        .collect()
}

#[test]
fn answered_questions_have_a_blank_line_between_them_unless_compact() {
    let block = answered_questions_output();

    let expanded = rendered_in(&block, &context(DisplayMode::Expanded));
    let first = expanded
        .iter()
        .position(|line| line.contains("Which runtime?"))
        .expect("first question");
    let second = expanded
        .iter()
        .position(|line| line.contains("Which port?"))
        .expect("second question");
    assert!(
        expanded[first + 1].contains("tokio"),
        "the answer stays on the next row: {expanded:?}"
    );
    assert!(
        expanded[second - 1].trim().is_empty(),
        "a blank row separates the questions: {expanded:?}"
    );
    assert!(
        expanded[second].contains("2."),
        "the second question keeps its number: {expanded:?}"
    );
    assert_eq!(second, first + 3, "{expanded:?}");

    let truncated = rendered_in(&block, &context(DisplayMode::Truncated));
    assert_eq!(truncated, expanded);

    let mut compact = context(DisplayMode::Expanded);
    compact.appearance.prompt.compact = true;
    let compact_lines = rendered_in(&block, &compact);
    let compact_first = compact_lines
        .iter()
        .position(|line| line.contains("Which runtime?"))
        .expect("compact first question");
    let compact_second = compact_lines
        .iter()
        .position(|line| line.contains("Which port?"))
        .expect("compact second question");
    assert_eq!(
        compact_second, compact_first + 2,
        "compact mode keeps the pairs adjacent: {compact_lines:?}"
    );
    assert!(
        compact_lines[compact_first + 1].contains("tokio"),
        "{compact_lines:?}"
    );

    let collapsed = rendered(&block, DisplayMode::Collapsed);
    assert_eq!(collapsed.lines().count(), 1, "{collapsed}");
    assert!(!collapsed.contains("Which runtime?"), "{collapsed}");
}
