use std::path::Path;

use xai_grok_hooks::discovery::DiscoveryOptions;
use xai_grok_hooks::error::HookError;
use xai_grok_hooks::trust::DisabledHooks;
use xai_grok_hooks::trust::Trust;
use xai_grok_workspace::hook_inputs::ProcessHookInputs;
use xai_grok_workspace::permission::resolution::load_managed_settings;

pub(crate) fn process_hook_inputs() -> ProcessHookInputs {
    ProcessHookInputs::read(crate::claude_import::import_marker())
}

/// For a session that dispatches the hooks it discovers.
/// Managed policy is read from disk so a pin changed after process start applies to the next session.
pub(crate) fn session_hook_inputs() -> (ProcessHookInputs, DisabledHooks) {
    let managed = load_managed_settings();
    ProcessHookInputs::read_with_disabled(&managed, crate::claude_import::import_marker())
}

#[cfg(test)]
mod session_hook_input_tests {
    #[test]
    fn each_session_reads_managed_policy_without_restarting() {
        let (_, first) = super::session_hook_inputs();
        let (_, second) = super::session_hook_inputs();
        assert_eq!(first.managed_only(), second.managed_only());
    }
}

/// Every session startup and mid-session reload loads hooks through this function.
/// This is the one place that chooses which hook sources to load.
pub(crate) fn discover_hooks(
    inputs: &ProcessHookInputs,
    git_root: Option<&Path>,
    compat: &xai_grok_tools::types::compat::CompatConfig,
    trust: Trust,
    user_hooks_trusted: bool,
) -> (xai_grok_hooks::discovery::HookRegistry, Vec<HookError>) {
    discover_hooks_at(
        inputs,
        git_root,
        None,
        compat,
        trust,
        user_hooks_trusted,
    )
}

/// `launch_dir` is the directory the session started in. Its `.grok/hooks` loads with the git root.
pub(crate) fn discover_hooks_at(
    inputs: &ProcessHookInputs,
    git_root: Option<&Path>,
    launch_dir: Option<&Path>,
    compat: &xai_grok_tools::types::compat::CompatConfig,
    trust: Trust,
    user_hooks_trusted: bool,
) -> (xai_grok_hooks::discovery::HookRegistry, Vec<HookError>) {
    xai_grok_hooks::discovery::assemble_hooks(
        inputs.config_layers(),
        DiscoveryOptions {
            git_root,
            grok_home: inputs.grok_home(),
            home: inputs.home(),
            compat: compat.hooks(),
            claude_import: inputs.claude_import(),
            trust,
            // A trusted folder already loads every hook. The flag matters when
            // the folder is untrusted and this session is headless.
            user_hooks_trusted,
            launch_dir,
        },
    )
}
