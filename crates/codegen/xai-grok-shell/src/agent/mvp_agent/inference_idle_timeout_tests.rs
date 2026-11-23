use super::resolve_inference_idle_timeout_secs;
use crate::agent::config::{ModelEntry, ModelInfo};
use crate::util::config::RemoteSettings;
use futures_util::StreamExt;
use std::time::Duration;

fn entry(slug: &str, timeout: Option<u64>) -> ModelEntry {
    let mut info = ModelInfo::fallback(slug);
    info.inference_idle_timeout_secs = timeout;
    ModelEntry {
        info,
        mtls_cert_dir: None,
        api_key: None,
        env_key: None,
        auth_provider: None,
        api_base_url: None,
    }
}

fn models_with(
    key: &str,
    slug: &str,
    timeout: Option<u64>,
) -> indexmap::IndexMap<String, ModelEntry> {
    let mut models = indexmap::IndexMap::new();
    models.insert(key.to_string(), entry(slug, timeout));
    models
}

#[test]
fn unset_idle_timeout_is_five_minutes_headless_and_ten_interactive() {
    let models = indexmap::IndexMap::new();
    assert_eq!(
        resolve_inference_idle_timeout_secs(&models, "grok-test", None, true),
        300
    );
    assert_eq!(
        resolve_inference_idle_timeout_secs(&models, "grok-test", None, false),
        600
    );
}

#[test]
fn per_model_and_remote_idle_timeouts_win_for_both_modes() {
    let models = models_with("catalog-key", "routed-id", Some(42));
    let remote = RemoteSettings {
        inference_idle_timeout_secs: Some(180),
        ..Default::default()
    };
    for headless in [true, false] {
        assert_eq!(
            resolve_inference_idle_timeout_secs(&models, "routed-id", Some(&remote), headless),
            42,
            "the per-model timeout wins, headless={headless}"
        );
        assert_eq!(
            resolve_inference_idle_timeout_secs(
                &indexmap::IndexMap::new(),
                "grok-test",
                Some(&remote),
                headless
            ),
            180,
            "remote settings win when the model leaves the timeout unset, headless={headless}"
        );
    }
}

#[test]
fn idle_timeout_floor_is_ten_seconds() {
    let models = models_with("grok-test", "grok-test", Some(3));
    let remote = RemoteSettings {
        inference_idle_timeout_secs: Some(4),
        ..Default::default()
    };
    for headless in [true, false] {
        assert_eq!(
            resolve_inference_idle_timeout_secs(&models, "grok-test", Some(&remote), headless),
            10,
            "a per-model timeout below the floor clamps, headless={headless}"
        );
        assert_eq!(
            resolve_inference_idle_timeout_secs(
                &indexmap::IndexMap::new(),
                "grok-test",
                Some(&remote),
                headless
            ),
            10,
            "a remote timeout below the floor clamps, headless={headless}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn headless_resolved_idle_timeout_stops_a_stalled_model_stream() {
    let secs =
        resolve_inference_idle_timeout_secs(&indexmap::IndexMap::new(), "grok-test", None, true);
    assert_eq!(secs, 300);
    let raw = futures_util::stream::pending::<
        Result<
            xai_grok_sampling_types::ChatCompletionChunk,
            xai_grok_sampling_types::SamplingError,
        >,
    >()
    .boxed();
    let events: Vec<_> = xai_grok_sampler::stream_chat_completions(
        raw,
        None,
        xai_grok_sampler::RequestId::from("headless-stall"),
        Duration::from_secs(secs),
    )
    .collect()
    .await;
    match events.last() {
        Some(xai_grok_sampler::SamplingEvent::Failed { error, .. }) => {
            assert_eq!(error.kind, xai_grok_sampler::SamplingErrorKind::IdleTimeout);
            assert!(
                error.message.contains("300s"),
                "the stalled stream must name the resolved headless deadline, got {}",
                error.message
            );
        }
        other => panic!("expected Failed(IdleTimeout), got {other:?}"),
    }
}
