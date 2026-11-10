//! `/delete`: delete this session's history (welcome, or dashboard when attached).
//! Extra text is an error. It does not delete the current session.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

const NO_ARGS: &str = "/delete takes no arguments.";

pub struct DeleteCommand;

impl SlashCommand for DeleteCommand {
    slash_meta! {
        name: "delete",
        description: "Delete this session",
        usage: "/delete",
        session_scoped: true,
    }

    fn submission_refusal(&self, args: &str, _voice_owns_prompt: bool) -> Option<&'static str> {
        (!args.trim().is_empty()).then_some(NO_ARGS)
    }

    fn run(&self, ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Error(NO_ARGS.to_string());
        }
        if ctx.session_id.is_none() {
            return CommandResult::Error("No active session to delete".into());
        }
        CommandResult::Action(Action::DeleteCurrentSession)
    }
}
