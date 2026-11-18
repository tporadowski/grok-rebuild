//! `/models` refreshes the available model list from the server.
//! It does not switch the session model. `/model` still does that.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

pub struct ModelsCommand;

impl SlashCommand for ModelsCommand {
    slash_meta! {
        name: "models",
        description: "Refresh the available model list from the server",
        usage: "/models",
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        if args.trim().is_empty() {
            CommandResult::Action(Action::RefreshModels)
        } else {
            CommandResult::Error("/models takes no arguments".to_string())
        }
    }
}

/// Decode an `x.ai/models/refresh` body. An error field wins over a count.
pub(crate) fn models_refresh_outcome(raw: &str) -> Result<usize, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| "could not refresh the model list".to_string())?;
    if let Some(error) = value.get("error").filter(|error| !error.is_null()) {
        let message = error.as_str().unwrap_or("could not refresh the model list");
        return Err(message.to_string());
    }
    value
        .get("result")
        .and_then(|result| result.get("models"))
        .and_then(|count| count.as_u64())
        .map(|count| count as usize)
        .ok_or_else(|| "could not refresh the model list".to_string())
}

pub(crate) fn models_refresh_notice(result: &Result<usize, String>) -> String {
    match result {
        Ok(1) => "Refreshed 1 model.".to_string(),
        Ok(count) => format!("Refreshed {count} models."),
        Err(error) => error.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_refresh_outcome_reads_the_count() {
        let count = models_refresh_outcome(r#"{"result":{"models":4}}"#).expect("count");
        assert_eq!(count, 4);
        assert_eq!(models_refresh_notice(&Ok(4)), "Refreshed 4 models.");
        assert_eq!(models_refresh_notice(&Ok(1)), "Refreshed 1 model.");
    }

    #[test]
    fn models_refresh_outcome_prefers_the_error() {
        let err = models_refresh_outcome(
            r#"{"result":{"models":2},"error":"could not refresh the model list"}"#,
        )
        .expect_err("error");
        assert_eq!(err, "could not refresh the model list");
        assert_eq!(
            models_refresh_notice(&Err(err)),
            "could not refresh the model list"
        );
    }
}
