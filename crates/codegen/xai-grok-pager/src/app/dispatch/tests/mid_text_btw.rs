//! `/btw` starts a side question only when the message begins with it.
//! A later `/btw` is sent to the main conversation as typed.

use super::*;
use crate::views::btw_overlay::BtwOverlayState;
use pretty_assertions::assert_eq;

const TYPED: &str = "explain the controller. /btw what is a WBC";

fn assert_fullscreen_side_question(effects: &[Effect], question: &str) {
    assert!(
        matches!(
            effects,
            [Effect::SendBtw { question: sent, minimal_request_id: None, .. }] if sent == question
        ),
        "expected exactly one fullscreen /btw send of {question:?}, got {effects:?}"
    );
}

fn assert_main_turn(effects: &[Effect], text: &str) {
    assert!(
        matches!(
            effects,
            [Effect::SendPrompt { text: sent, .. }] if sent == text
        ),
        "expected the typed text on the main turn, got {effects:?}"
    );
}

#[test]
fn leading_btw_still_starts_a_side_question() {
    let mut app = test_app_with_agent();
    let id = AgentId(0);

    let effects = dispatch(Action::SendPrompt("/btw what is a WBC".into()), &mut app);

    assert_fullscreen_side_question(&effects, "what is a WBC");
    let agent = test_agent(&app, id);
    assert!(
        matches!(
            &agent.btw_state,
            Some(BtwOverlayState::Loading { question }) if question == "what is a WBC"
        ),
        "{:?}",
        agent.btw_state
    );
    assert_eq!("", agent.prompt.text());
}

#[test]
fn mid_text_btw_goes_to_the_main_turn() {
    let mut app = test_app_with_agent();
    let id = AgentId(0);

    let effects = dispatch(Action::SendPrompt(TYPED.to_owned()), &mut app);

    assert_main_turn(&effects, TYPED);
    assert!(test_agent(&app, id).btw_state.is_none());
    assert_eq!("", test_agent(&app, id).prompt.text());
}

#[test]
fn mid_text_btw_while_turn_running_stays_a_prompt() {
    let mut app = test_app_with_agent();
    let id = AgentId(0);
    app.agents.get_mut(&id).unwrap().session.state = AgentState::TurnRunning;

    let effects = dispatch(Action::SendPrompt(TYPED.to_owned()), &mut app);

    assert_main_turn(&effects, TYPED);
    let agent = test_agent(&app, id);
    assert!(agent.btw_state.is_none());
    assert!(agent.session.pending_prompts.is_empty());
}

#[test]
fn mid_text_btw_history_keeps_typed_text() {
    let mut app = test_app_with_agent();

    dispatch(Action::SendPrompt(TYPED.to_owned()), &mut app);

    assert_eq!(
        vec![TYPED.to_owned()],
        test_agent(&app, AgentId(0)).session.prompt_history
    );
}

#[test]
fn mid_text_btw_sends_composer_images() {
    let mut app = test_app_with_agent();
    let id = AgentId(0);
    {
        let agent = app.agents.get_mut(&id).unwrap();
        agent.prompt.set_text("look /btw q ");
        let end = agent.prompt.text().len();
        agent.prompt.set_cursor(end);
        agent
            .prompt
            .insert_image(crate::app::agent_view::test_fixtures::test_pasted_image())
            .expect("chip");
    }
    let composed = test_agent(&app, id).prompt.text().to_owned();

    let effects = dispatch(Action::SendPrompt(composed.clone()), &mut app);

    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, Effect::SendBtw { .. })),
        "a /btw after other words is not a side question: {effects:?}"
    );
    match effects.as_slice() {
        [Effect::SendPromptBlocks { blocks, .. }] => {
            let Some(acp::ContentBlock::Text(text)) = blocks.first() else {
                panic!("first block must be the typed text, got {blocks:?}");
            };
            assert!(
                text.text.contains("look /btw q"),
                "the token stays in the main turn: {text:?}"
            );
            assert_eq!(
                blocks
                    .iter()
                    .filter(|block| matches!(block, acp::ContentBlock::Image(_)))
                    .count(),
                1
            );
        }
        other => panic!("expected the image on the main turn, got {other:?}"),
    }
    assert!(test_agent(&app, id).btw_state.is_none());
    let agent = test_agent(&app, id);
    assert!(agent.prompt.images.is_empty());
    assert_eq!("", agent.prompt.text());
}

/// Paste first, then type `/btw q`: the message begins with the chip, so `/btw` stays typed text.
#[test]
fn leading_image_chip_then_btw_stays_on_the_main_turn() {
    let mut app = test_app_with_agent();
    let id = AgentId(0);
    {
        let agent = app.agents.get_mut(&id).unwrap();
        agent
            .prompt
            .insert_image(crate::app::agent_view::test_fixtures::test_pasted_image())
            .expect("chip");
        agent.prompt.textarea.insert_str("/btw q");
    }
    let composed = test_agent(&app, id).prompt.text().to_owned();
    assert_eq!("[Image #1] /btw q", composed);

    let effects = dispatch(Action::SendPrompt(composed.clone()), &mut app);

    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, Effect::SendBtw { .. })),
        "{effects:?}"
    );
    match effects.as_slice() {
        [Effect::SendPromptBlocks { blocks, .. }] => {
            let Some(acp::ContentBlock::Text(text)) = blocks.first() else {
                panic!("first block must be the typed text, got {blocks:?}");
            };
            assert!(
                text.text.contains("/btw q"),
                "the token stays where it was typed: {text:?}"
            );
            assert_eq!(
                blocks
                    .iter()
                    .filter(|block| matches!(block, acp::ContentBlock::Image(_)))
                    .count(),
                1
            );
        }
        other => panic!("expected the image on the main turn, got {other:?}"),
    }
    assert!(test_agent(&app, id).btw_state.is_none());
    let agent = test_agent(&app, id);
    assert!(agent.prompt.images.is_empty());
    assert_eq!("", agent.prompt.text());
}

/// An image chip, then an unknown command, then `/btw`, is still one main-turn message.
#[test]
fn leading_image_chip_then_unknown_command_then_btw_stays_typed() {
    let mut app = test_app_with_agent();
    let id = AgentId(0);
    {
        let agent = app.agents.get_mut(&id).unwrap();
        agent
            .prompt
            .insert_image(crate::app::agent_view::test_fixtures::test_pasted_image())
            .expect("chip");
        agent.prompt.textarea.insert_str("/nope hi /btw q");
    }
    let composed = test_agent(&app, id).prompt.text().to_owned();
    assert_eq!("[Image #1] /nope hi /btw q", composed);

    let effects = dispatch(Action::SendPrompt(composed.clone()), &mut app);

    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, Effect::SendBtw { .. })),
        "{effects:?}"
    );
    match effects.as_slice() {
        [Effect::SendPromptBlocks { blocks, .. }] => {
            let Some(acp::ContentBlock::Text(text)) = blocks.first() else {
                panic!("first block must be the typed text, got {blocks:?}");
            };
            assert!(
                text.text.contains("/nope hi /btw q"),
                "both tokens stay as typed: {text:?}"
            );
        }
        other => panic!("expected the typed line on the main turn, got {other:?}"),
    }
    assert!(test_agent(&app, id).btw_state.is_none());
}

#[test]
fn leading_unknown_command_with_btw_is_not_hoisted() {
    let mut app = test_app_with_agent();

    let effects = dispatch(Action::SendPrompt("/nope hi /btw q".to_owned()), &mut app);

    assert!(
        matches!(
            effects.as_slice(),
            [Effect::SendPrompt { text, .. }] if text == "/nope hi /btw q"
        ),
        "{effects:?}"
    );
}

#[test]
fn literal_follow_up_with_btw_is_not_hoisted() {
    let mut app = test_app_with_agent();

    let effects = dispatch(Action::SubmitFollowUp("prose /btw q".to_owned()), &mut app);

    assert!(
        matches!(
            effects.as_slice(),
            [Effect::SendPrompt { text, .. }] if text == "prose /btw q"
        ),
        "{effects:?}"
    );
}

#[test]
fn mid_text_other_builtin_still_passes_through() {
    let mut app = test_app_with_agent();

    let effects = dispatch(Action::SendPrompt("great /compact go".to_owned()), &mut app);

    assert!(
        matches!(
            effects.as_slice(),
            [Effect::SendPrompt { text, .. }] if text == "great /compact go"
        ),
        "{effects:?}"
    );
}
