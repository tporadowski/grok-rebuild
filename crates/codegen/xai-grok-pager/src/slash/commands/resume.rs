//! `/resume`: open the session picker overlay to resume a previous session.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

pub struct ResumeCommand;

impl SlashCommand for ResumeCommand {
    slash_meta! {
        name: "resume",
        description: "Resume a previous session",
        usage: "/resume [id or title]",
        takes_args: true,
        arg_placeholder: "[id or title]",
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let query = resume_picker_query(args);
        CommandResult::Action(Action::ShowSessionPicker(query))
    }
}

/// Trimmed `/resume` argument. Empty opens an unfiltered picker.
pub(crate) fn resume_picker_query(args: &str) -> Option<String> {
    let trimmed = args.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_query_keeps_an_id_or_title_and_drops_blank_args() {
        assert_eq!(resume_picker_query("").as_deref(), None);
        assert_eq!(resume_picker_query("   ").as_deref(), None);
        assert_eq!(
            resume_picker_query("  fix-login-bug ").as_deref(),
            Some("fix-login-bug")
        );
    }
}
