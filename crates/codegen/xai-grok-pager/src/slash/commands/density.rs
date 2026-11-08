//! `/density`: choose how much of each tool call the transcript shows.

use crate::app::actions::Action;
use crate::appearance::ToolCallDensity;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

pub struct DensityCommand;

/// No argument cycles the five modes. A word sets that mode.
pub fn resolve_density_command(
    current: ToolCallDensity,
    args: &str,
) -> Result<ToolCallDensity, String> {
    let args = args.trim();
    if args.is_empty() {
        return Ok(current.next());
    }
    ToolCallDensity::parse(args).ok_or_else(|| {
        format!(
            "Usage: /density [detailed|compact-shells|compact-ungrouped|compact-grouped|compact-all-grouped] (got {args})"
        )
    })
}

impl SlashCommand for DensityCommand {
    slash_meta! {
        name: "density",
        description: "How much detail tool calls show in the transcript",
        usage: "/density [detailed|compact-shells|compact-ungrouped|compact-grouped|compact-all-grouped]",
        arg_placeholder: "detailed/compact-…",
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        match resolve_density_command(crate::appearance::cache::load_tool_call_density(), args) {
            Ok(mode) => CommandResult::Action(Action::SetToolCallDensity(mode)),
            Err(message) => CommandResult::Error(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_command_cycles_and_accepts_a_mode() {
        assert_eq!(
            resolve_density_command(ToolCallDensity::Detailed, "").unwrap(),
            ToolCallDensity::CompactShells
        );
        assert_eq!(
            resolve_density_command(ToolCallDensity::CompactAllGrouped, "  ").unwrap(),
            ToolCallDensity::Detailed
        );
        assert_eq!(
            resolve_density_command(ToolCallDensity::Detailed, "grouped").unwrap(),
            ToolCallDensity::CompactGrouped
        );
        assert_eq!(
            resolve_density_command(ToolCallDensity::CompactGrouped, "compact-all-grouped")
                .unwrap(),
            ToolCallDensity::CompactAllGrouped
        );
        assert!(resolve_density_command(ToolCallDensity::Detailed, "tight").is_err());
    }
}
