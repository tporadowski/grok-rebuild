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
