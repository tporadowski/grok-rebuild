//! `/timestamps`: choose off, minimal, or on. No argument cycles those three.

use crate::appearance::TimestampMode;
use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

pub struct TimestampsCommand;

/// No argument cycles off → minimal → on. A word sets that mode.
pub fn resolve_timestamps_command(current: TimestampMode, args: &str) -> Result<TimestampMode, String> {
    let args = args.trim();
    if args.is_empty() {
        return Ok(current.next());
    }
    TimestampMode::parse(args).ok_or_else(|| {
        format!("Usage: /timestamps [off|minimal|on] (got {args})")
    })
}

impl SlashCommand for TimestampsCommand {
    slash_meta! {
        name: "timestamps",
        description: "Clock next to messages: off, minimal, or on",
        usage: "/timestamps [off|minimal|on]",
        arg_placeholder: "off/minimal/on",
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        match resolve_timestamps_command(crate::appearance::cache::load_timestamps(), args) {
            Ok(mode) => CommandResult::Action(Action::SetTimestamps(mode)),
            Err(message) => CommandResult::Error(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_command_cycles_and_accepts_a_mode() {
        assert_eq!(
            resolve_timestamps_command(TimestampMode::Off, "").unwrap(),
            TimestampMode::Minimal
        );
        assert_eq!(
            resolve_timestamps_command(TimestampMode::Minimal, "  ").unwrap(),
            TimestampMode::On
        );
        assert_eq!(
            resolve_timestamps_command(TimestampMode::On, "off").unwrap(),
            TimestampMode::Off
        );
        assert_eq!(
            resolve_timestamps_command(TimestampMode::Off, "minimal").unwrap(),
            TimestampMode::Minimal
        );
        assert_eq!(
            resolve_timestamps_command(TimestampMode::Minimal, "true").unwrap(),
            TimestampMode::On
        );
        assert!(resolve_timestamps_command(TimestampMode::On, "sometimes").is_err());
    }
}
