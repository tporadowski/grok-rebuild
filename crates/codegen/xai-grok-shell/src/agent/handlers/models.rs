//! `x.ai/models/list`: the model catalog for one-shot consumers.

use agent_client_protocol::{self as acp};

use super::super::mvp_agent::MvpAgent;
use crate::session::ExtMethodResult;

/// Returns the model state after a bounded wait for the first catalog.
/// Process chat mode serves the chat catalog, as `initialize` does.
/// `x.ai/models/refresh` fetches `/v1/models` and publishes `x.ai/models/update`.
pub(crate) async fn handle(
    agent: &MvpAgent,
    args: &acp::ExtRequest,
) -> Result<acp::ExtResponse, acp::Error> {
    if args.method.as_ref() == "x.ai/models/refresh" {
        return refresh(agent).await;
    }
    let state = if crate::agent::chat_modes::process_chat_mode_enabled() {
        agent.chat_modes.model_state().await
    } else {
        agent
            .models_manager
            .wait_for_first_catalog(agent.models_manager.is_models_fetch_enabled())
            .await;
        agent.model_state(None)
    };
    ExtMethodResult::success(state)
        .to_ext_response()
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))
}

async fn refresh(agent: &MvpAgent) -> Result<acp::ExtResponse, acp::Error> {
    let outcome = agent.models_manager.refresh_from_server().await;
    let body = match outcome {
        Ok(models) => ExtMethodResult::success(serde_json::json!({ "models": models })),
        Err(message) => ExtMethodResult::failure(message),
    };
    body.to_ext_response()
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))
}
