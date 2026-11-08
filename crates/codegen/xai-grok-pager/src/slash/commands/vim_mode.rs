//! `/vim-mode`: set `[keys] vim` to `off` or `nav`.
//!
//! With no argument it toggles. `nav` (and `on`) turns scrollback vim keys on. `off` turns them off.
//! `[ui] vim_mode = true` still means `nav` when `[keys] vim` is absent.
//!
//! When off (the default), bare-letter and Shift+letter keys in the scrollback jump focus to the prompt so the letter is typed into the textarea.
//! That covers j/k, h/l, g/G, y/Y, o/O, r, x, e/E, L/H, and the `i` insert alternative.
//! Arrow/Tab/Esc/Space/PgUp/PgDn and all Ctrl+letter bindings stay active in both modes.

use crate::app::actions::Action;
use crate::appearance::VimKeys;
use crate::slash::command::{ArgItem, CommandExecCtx, CommandResult, SlashCommand, slash_meta};

pub struct VimModeCommand;

impl SlashCommand for VimModeCommand {
    slash_meta! {
        name: "vim-mode",
        description: "Vim scrollback keys: off or nav. No argument toggles.",
        usage: "/vim-mode [off|nav]",
        takes_args: true,
    }

    fn suggest_args(
        &self,
        _ctx: &crate::slash::command::AppCtx,
        _args_query: &str,
    ) -> Option<Vec<ArgItem>> {
        Some(vec![
            ArgItem {
                display: "off".into(),
                match_text: "off".into(),
                insert_text: "off".into(),
                description: "Arrow keys only in the scrollback".into(),
            },
            ArgItem {
                display: "nav".into(),
                match_text: "nav".into(),
                insert_text: "nav".into(),
                description: "j/k, h/l, g/G, and the other scrollback motions".into(),
            },
        ])
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        vim_mode_command(args)
    }
}

pub(crate) fn vim_mode_command(args: &str) -> CommandResult {
    let arg = args.trim();
    if arg.is_empty() {
        return CommandResult::Action(Action::ToggleVimMode);
    }
    match VimKeys::parse(arg) {
        Some(VimKeys::Off) => CommandResult::Action(Action::SetVimMode(false)),
        Some(VimKeys::Nav) => CommandResult::Action(Action::SetVimMode(true)),
        Some(VimKeys::All) => CommandResult::Error(
            "vim = \"all\" is set in config.toml. /vim-mode sets off or nav.".into(),
        ),
        None => CommandResult::Error("Usage: /vim-mode [off|nav]".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vim_mode_toggles_or_sets_off_and_nav() {
        assert!(matches!(
            vim_mode_command(""),
            CommandResult::Action(Action::ToggleVimMode)
        ));
        assert!(matches!(
            vim_mode_command("nav"),
            CommandResult::Action(Action::SetVimMode(true))
        ));
        assert!(matches!(
            vim_mode_command("on"),
            CommandResult::Action(Action::SetVimMode(true))
        ));
        assert!(matches!(
            vim_mode_command("off"),
            CommandResult::Action(Action::SetVimMode(false))
        ));
        assert!(matches!(
            vim_mode_command("sideways"),
            CommandResult::Error(_)
        ));
        assert!(matches!(vim_mode_command("all"), CommandResult::Error(_)));
    }
}
