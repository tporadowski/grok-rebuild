use pretty_assertions::assert_eq;

use super::{contains_goal_command_token, hoist_mid_text_command, token_is_armed_inline};
use crate::acp::model_state::ModelState;
use crate::slash::SlashController;
use crate::slash::commands;
use crate::slash::registry::CommandRegistry;
use crate::slash::scan_inline_slash_tokens;
use agent_client_protocol as acp;

fn builtins() -> CommandRegistry {
    CommandRegistry::new(commands::builtin_commands())
}

fn with_goal() -> CommandRegistry {
    let mut registry = builtins();
    registry.set_acp_commands(&[acp::AvailableCommand::new(
        "goal".to_string(),
        "Set a goal".to_string(),
    )]);
    registry
}

fn hoist(text: &str) -> Option<String> {
    hoist_mid_text_command(text, &builtins())
}

fn first_token(text: &str) -> crate::slash::InlineSlashToken {
    scan_inline_slash_tokens(text, 0)
        .into_iter()
        .next()
        .expect("expected a slash token")
}

/// `/btw` does not opt into hoist. A token that already starts the line is still
/// `None` here; the submit path runs that as a slash command without this rewrite.
#[test]
fn mid_text_btw_is_not_rewritten() {
    for text in [
        "explain the controller. /btw what is a WBC",
        "prose. /btw   q",
        "prose /btw",
        "prose /btw   ",
        "line1\n/btw q\nline2",
        "a /btw b /btw c",
        " /btw q",
        "/btw q",
        "/nope hi /btw q",
    ] {
        assert_eq!(None, hoist(text), "{text:?}");
    }
}

#[test]
fn empty_question_is_none() {
    for text in ["/btw", " /btw", "/btw   "] {
        assert_eq!(None, hoist(text), "{text:?}");
    }
}

#[test]
fn punctuated_or_uppercase_token_misses() {
    for text in ["hi /btw, q", "hi /btw: q", "hi /BTW q"] {
        assert_eq!(None, hoist(text), "{text:?}");
    }
}

#[test]
fn path_and_url_slashes_miss() {
    for text in ["see docs/btw", "open https://x.ai/btw now"] {
        assert_eq!(None, hoist(text), "{text:?}");
    }
}

#[test]
fn non_opted_in_builtin_misses() {
    assert_eq!(None, hoist("great /compact go"));
}

#[test]
fn unknown_command_misses() {
    assert_eq!(None, hoist("hi /nope q"));
}

#[test]
fn detects_mid_text_goal_only_when_advertised() {
    let registry = with_goal();
    assert!(contains_goal_command_token("ctx\n/goal do it", &registry));
    assert!(contains_goal_command_token("please /goal now", &registry));
    assert!(contains_goal_command_token(
        "context /goal investigate /btw why",
        &registry
    ));
    assert!(contains_goal_command_token(
        "context /btw why /goal investigate",
        &registry
    ));
    assert!(!contains_goal_command_token(
        "please /compact now",
        &registry
    ));
    assert!(!contains_goal_command_token(
        "please /goal now",
        &builtins()
    ));
}

#[test]
fn mid_text_goal_is_unarmed_leading_goal_is_armed() {
    let registry = with_goal();
    let cmd = registry.get_for_dispatch("goal").expect("goal");
    let mid = "ctx\n/goal do it";
    assert!(!token_is_armed_inline(mid, &first_token(mid), cmd.as_ref()));
    let leading = "/goal do it";
    assert!(token_is_armed_inline(
        leading,
        &first_token(leading),
        cmd.as_ref()
    ));
    let indented = "  /goal do it";
    assert!(token_is_armed_inline(
        indented,
        &first_token(indented),
        cmd.as_ref()
    ));
}

#[test]
fn btw_is_armed_only_when_it_begins_the_message() {
    let registry = builtins();
    let cmd = registry.get_for_dispatch("btw").expect("btw");
    let mid = "please /btw q";
    assert!(!token_is_armed_inline(mid, &first_token(mid), cmd.as_ref()));
    let leading = "/btw q";
    assert!(token_is_armed_inline(
        leading,
        &first_token(leading),
        cmd.as_ref()
    ));
    let indented = "  /btw q";
    assert!(token_is_armed_inline(
        indented,
        &first_token(indented),
        cmd.as_ref()
    ));
    let compact = "great /compact go";
    assert!(!token_is_armed_inline(
        compact,
        &first_token(compact),
        registry
            .get_for_dispatch("compact")
            .expect("compact")
            .as_ref()
    ));
}

#[test]
fn composer_highlights_leading_goal_not_mid_text_goal() {
    let mut ctrl = SlashController::with_builtins(std::path::PathBuf::from("."));
    ctrl.registry_mut()
        .set_acp_commands(&[acp::AvailableCommand::new(
            "goal".to_string(),
            "Set a goal".to_string(),
        )]);
    let models = ModelState::default();
    assert!(
        ctrl.recognized_token_ranges("ctx\n/goal do it", &models)
            .is_empty()
    );
    assert_eq!(
        ctrl.recognized_token_ranges("/goal do it", &models),
        vec![0..5]
    );
}
