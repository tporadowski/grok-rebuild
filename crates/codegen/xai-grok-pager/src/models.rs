//! `grok models` subcommand.

use anyhow::Result;
use tokio_util::sync::CancellationToken;
use xai_grok_shell::agent::config::Config as AgentConfig;
use xai_grok_shell::cli_models::{AuthStatus, list_models};

use crate::client_identity::{PAGER_CLIENT_TYPE, PAGER_CLIENT_VERSION};

fn outln(line: impl std::fmt::Display) -> Result<()> {
    // `println!` panics when stdout closes (`grok models | head`) or the device is full.
    Ok(crate::util::println_stdout(line)?)
}

pub async fn list_available_models(agent_config: &AgentConfig) -> Result<()> {
    match AuthStatus::resolve(agent_config) {
        AuthStatus::ApiKey => outln("You are using XAI_API_KEY.")?,
        AuthStatus::LoggedIn(host) => outln(format!("You are logged in with {host}."))?,
        AuthStatus::ModelCredentials(model) => {
            outln(format!("Model '{model}' is using its own API key."))?;
        }
        AuthStatus::DeploymentKey => outln("You are authenticated via deployment key.")?,
        AuthStatus::NotAuthenticated => outln("You are not authenticated.")?,
    }
    outln("")?;

    let cancel = CancellationToken::new();
    xai_grok_telemetry::startup::mark_utility_process();
    let spawned =
        crate::acp::spawn::spawn_grok_shell(agent_config.clone(), &cancel, None, true).await?;
    // Cancel and join on every return path, including the `?` below
    let _agent_guard =
        crate::acp::spawn::AgentShutdownGuard::new(cancel.clone(), Some(spawned.thread_handle));

    let state = list_models(&spawned.channel.tx, PAGER_CLIENT_TYPE, PAGER_CLIENT_VERSION).await?;

    outln(format!("Default model: {}", state.current_model_id.0))?;
    outln("")?;
    outln("Available models:")?;
    for m in state.available_models {
        if m.model_id == state.current_model_id {
            outln(format!("  * {} (default)", m.model_id.0))?;
        } else {
            outln(format!("  - {}", m.model_id.0))?;
        }
    }

    Ok(())
}
