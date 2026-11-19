use xai_grok_sampling_types::conversation::ConversationItem;

use super::support::{build_actor, running_task_stub};
use crate::session::SwitchContextWindow;
use crate::session::compaction_config::AsyncCompactionCache;

const SWITCH_TARGET_LABEL: &str = "Aurora";

fn head_text(conv: &[ConversationItem]) -> String {
    let Some(ConversationItem::System(sys)) = conv.first() else {
        panic!(
            "conversation must start with a System item, got {:?}",
            conv.first()
        );
    };
    sys.content.as_ref().to_owned()
}

#[tokio::test(flavor = "current_thread")]
async fn session_switches_when_its_model_leaves_the_available_list() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (actor, _gateway_rx) = build_actor().await;
            let endpoints = actor.models_manager.endpoints();
            actor.models_manager.insert_test_entry(
                "kept-model",
                crate::agent::config::ModelEntry::fallback("kept-model", &endpoints),
            );
            actor
                .models_manager
                .set_current_model_id(agent_client_protocol::ModelId::new("kept-model"));
            let replacement = actor.models_manager.current_model_id();
            assert_eq!(replacement.0.as_ref(), "kept-model");
            let mut sampling = actor
                .chat_state_handle
                .get_sampling_config()
                .await
                .expect("the session has a sampling config");
            sampling.model = "removed-from-the-list".to_owned();
            actor.chat_state_handle.update_sampling_config(sampling);

            assert!(
                actor
                    .switch_session_if_its_model_left_the_catalog()
                    .await
            );
            let switched = actor
                .chat_state_handle
                .get_sampling_config()
                .await
                .expect("sampling config remains");
            assert_eq!(switched.model, replacement.0.as_ref());

            assert!(
                !actor
                    .switch_session_if_its_model_left_the_catalog()
                    .await,
                "a model that is still listed stays on the session"
            );
            let stayed = actor.chat_state_handle.get_sampling_config().await.unwrap();
            assert_eq!(stayed.model, replacement.0.as_ref());
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn model_switch_relabels_live_agent_and_system_head() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (actor, _gateway_rx) = build_actor().await;
            actor.chat_state_handle.replace_conversation(vec![
                ConversationItem::system("spawn-time prompt"),
                ConversationItem::user("hi"),
            ]);

            actor
                .handle_set_session_model(crate::session::SessionModelSwitch {
                    sampling_config: xai_grok_sampler::SamplerConfig {
                        model: "switch-target".to_owned(),
                        context_window: 256_000,
                        ..xai_grok_sampler::SamplerConfig::default()
                    },
                    use_concise: false,
                    is_family_switch: false,
                    apply_prompt_override: true,
                    skip_prompt_rewrite: false,
                    auto_compact_threshold_percent: 85,
                    system_prompt_label: SWITCH_TARGET_LABEL.to_owned(),
                    context_window_selection: SwitchContextWindow::Set(None),
                    supported_context_windows: Vec::new(),
                })
                .await
                .expect("model switch succeeds");

            let conv = actor.chat_state_handle.get_conversation().await;
            let head = head_text(&conv);
            assert!(
                head.contains(SWITCH_TARGET_LABEL),
                "system head must render the new model's label, got: {head:.120}"
            );
            let agent = actor.agent.borrow();
            assert_eq!(
                SWITCH_TARGET_LABEL,
                agent.prompt_context().system_prompt_label
            );
            assert_eq!(agent.system_prompt(), head);
            assert_eq!(2, conv.len(), "the switch swaps only the head");
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn relabel_skipped_mid_turn_keeps_prefire_cache() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (actor, _gateway_rx) = build_actor().await;
            actor.compaction.prefire.store(AsyncCompactionCache {
                note1: "NOTE1".to_owned(),
                prefix_len: 1,
                fingerprint: 42,
                model_slug: "switch-target".to_owned(),
                pass1_latency_ms: 5,
            });
            actor.state.lock().await.running_task = Some(running_task_stub("running"));

            actor
                .relabel_agent_system_prompt(SWITCH_TARGET_LABEL.to_owned())
                .await;

            assert!(
                actor.compaction.prefire.has_cache(),
                "a skipped relabel must not drop a prefire built on the still-live prompt"
            );
            assert_eq!(
                xai_grok_agent::DEFAULT_SYSTEM_PROMPT_LABEL,
                actor.agent.borrow().prompt_context().system_prompt_label
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn zero_turn_rebuild_renders_switch_target_label() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (actor, _gateway_rx) = build_actor().await;

            actor
                .handle_rebuild_agent_for_definition(
                    xai_grok_agent::AgentDefinition::default_grok_build(),
                    SWITCH_TARGET_LABEL.to_owned(),
                )
                .await
                .expect("zero-turn rebuild succeeds");

            let conv = actor.chat_state_handle.get_conversation().await;
            let head = head_text(&conv);
            assert!(
                head.contains(SWITCH_TARGET_LABEL),
                "rebuilt head must render the new model's label, got: {head:.120}"
            );
            let agent = actor.agent.borrow();
            assert_eq!(
                SWITCH_TARGET_LABEL,
                agent.prompt_context().system_prompt_label
            );
            assert_eq!(agent.system_prompt(), head);
        })
        .await;
}
