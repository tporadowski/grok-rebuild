use crate::util::config::RemoteSettings;
use toml::Value as TomlValue;

pub const ENV_SHOW_THINKING_BLOCKS: &str = "GROK_SHOW_THINKING_BLOCKS";

#[cfg(test)]
static SHOW_THINKING_BLOCKS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Shared precedence core for `[ui]` bool flags: requirement > env > `[ui].<ui_key>` config > managed > remote (already extracted) > `default`.
fn resolve_ui_bool(
    env_var: &str,
    ui_key: &str,
    default: bool,
    requirements: Option<&TomlValue>,
    user: Option<&TomlValue>,
    managed: Option<&TomlValue>,
    remote_value: Option<bool>,
) -> crate::agent::config::Resolved<bool> {
    use crate::agent::config::BoolFlag;
    let from_toml =
        |v: Option<&TomlValue>| -> Option<bool> { v?.get("ui")?.get(ui_key)?.as_bool() };
    BoolFlag::env(env_var)
        .requirement(from_toml(requirements))
        .config(from_toml(user))
        .managed(from_toml(managed))
        .feature_flag(remote_value)
        .default(default)
        .resolve()
}

pub fn resolve_show_thinking_blocks(
    requirements: Option<&TomlValue>,
    user: Option<&TomlValue>,
    managed: Option<&TomlValue>,
    remote: Option<&RemoteSettings>,
) -> crate::agent::config::Resolved<bool> {
    resolve_ui_bool(
        ENV_SHOW_THINKING_BLOCKS,
        "show_thinking_blocks",
        true,
        requirements,
        user,
        managed,
        remote.and_then(|r| r.show_thinking_blocks),
    )
}

pub const ENV_GROUP_TOOL_VERBS: &str = "GROK_GROUP_TOOL_VERBS";

#[cfg(test)]
static GROUP_TOOL_VERBS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Resolve whether the TUI folds runs of consecutive non-destructive tool calls (reads/searches/lists) into one transcript row.
/// Remote `Some(false)` is the kill switch.
pub fn resolve_group_tool_verbs(
    requirements: Option<&TomlValue>,
    user: Option<&TomlValue>,
    managed: Option<&TomlValue>,
    remote: Option<&RemoteSettings>,
) -> crate::agent::config::Resolved<bool> {
    resolve_ui_bool(
        ENV_GROUP_TOOL_VERBS,
        "group_tool_verbs",
        true,
        requirements,
        user,
        managed,
        remote.and_then(|r| r.group_tool_verbs),
    )
}

pub const ENV_TOOL_CALL_DENSITY: &str = "GROK_TOOL_CALL_DENSITY";

/// One config tier's view of the density family. `None` means that tier did not set the key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DensityTier {
    pub density: Option<crate::agent::config::ToolCallDensity>,
    pub group_tool_verbs: Option<bool>,
    pub collapsed_edit_blocks: Option<bool>,
}

/// The fold that should be shown after the density family is combined across tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DensityFold {
    pub density: crate::agent::config::ToolCallDensity,
    pub group_tool_verbs: bool,
    pub collapsed_edit_blocks: bool,
}

/// `tiers` is highest priority first.
///
/// A legacy flag counts only from a tier at least as high as the tier that set
/// `tool_call_density`. A lower tier's `group_tool_verbs = false` does not turn
/// folding off against a higher tier's density, and a lower tier's
/// `collapsed_edit_blocks` does not force edit collapse against it. On the
/// winning tier, `group_tool_verbs = false` still turns every fold off and
/// `collapsed_edit_blocks = true` still forces edit collapse.
pub fn combine_density_tiers(tiers: &[DensityTier]) -> DensityFold {
    let density_at = tiers.iter().position(|tier| tier.density.is_some());
    let counts = |index: usize| density_at.is_none_or(|density| index <= density);
    let group_tool_verbs = tiers
        .iter()
        .enumerate()
        .find_map(|(index, tier)| counts(index).then_some(tier.group_tool_verbs).flatten())
        .unwrap_or(true);
    let collapsed_edit_blocks = tiers
        .iter()
        .enumerate()
        .find_map(|(index, tier)| counts(index).then_some(tier.collapsed_edit_blocks).flatten())
        .unwrap_or(false);
    DensityFold {
        density: tiers.iter().find_map(|tier| tier.density).unwrap_or_default(),
        group_tool_verbs,
        collapsed_edit_blocks,
    }
}

/// Requirement, then env, then user, then managed, then remote.
/// Env is `GROK_TOOL_CALL_DENSITY`, `GROK_GROUP_TOOL_VERBS`, and `GROK_COLLAPSED_EDIT_BLOCKS`.
pub fn resolve_density_fold(
    requirements: Option<&TomlValue>,
    user: Option<&TomlValue>,
    managed: Option<&TomlValue>,
    remote: Option<&RemoteSettings>,
) -> DensityFold {
    let tier = |value: Option<&TomlValue>| DensityTier {
        density: density_from_toml(value),
        group_tool_verbs: bool_from_toml(value, "group_tool_verbs"),
        collapsed_edit_blocks: bool_from_toml(value, "collapsed_edit_blocks"),
    };
    let env_density = std::env::var(ENV_TOOL_CALL_DENSITY)
        .ok()
        .and_then(|raw| crate::agent::config::ToolCallDensity::parse(&raw));
    combine_density_tiers(&[
        tier(requirements),
        DensityTier {
            density: env_density,
            group_tool_verbs: xai_grok_config::env_bool(ENV_GROUP_TOOL_VERBS),
            collapsed_edit_blocks: xai_grok_config::env_bool(ENV_COLLAPSED_EDIT_BLOCKS),
        },
        tier(user),
        tier(managed),
        DensityTier {
            density: None,
            group_tool_verbs: remote.and_then(|settings| settings.group_tool_verbs),
            collapsed_edit_blocks: remote.and_then(|settings| settings.collapsed_edit_blocks),
        },
    ])
}

fn bool_from_toml(value: Option<&TomlValue>, key: &str) -> Option<bool> {
    value?.get("ui")?.get(key)?.as_bool()
}

fn density_from_toml(
    value: Option<&TomlValue>,
) -> Option<crate::agent::config::ToolCallDensity> {
    value?
        .get("ui")?
        .get("tool_call_density")?
        .as_str()
        .and_then(crate::agent::config::ToolCallDensity::parse)
}

pub const ENV_COLLAPSED_EDIT_BLOCKS: &str = "GROK_COLLAPSED_EDIT_BLOCKS";

#[cfg(test)]
static COLLAPSED_EDIT_BLOCKS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Resolve whether the TUI shows Edit tool calls as a collapsed one-line `+N/-M` diffstat summary by default (expand for the diff).
/// This is a rollout flag: off keeps the legacy expanded-diff view.
pub fn resolve_collapsed_edit_blocks(
    requirements: Option<&TomlValue>,
    user: Option<&TomlValue>,
    managed: Option<&TomlValue>,
    remote: Option<&RemoteSettings>,
) -> crate::agent::config::Resolved<bool> {
    resolve_ui_bool(
        ENV_COLLAPSED_EDIT_BLOCKS,
        "collapsed_edit_blocks",
        false,
        requirements,
        user,
        managed,
        remote.and_then(|r| r.collapsed_edit_blocks),
    )
}

/// When enabled, the pager registers `Ctrl+R` (scrollback-focused only) so the user can flip terminal mouse capture. Turning capture off hands selection back to the terminal for native click-drag copy/paste.
/// Precedence: `GROK_MOUSE_REPORTING_TOGGLE` env > `[ui] mouse_reporting_toggle` > the parsed [`UiConfig`] field > default (`false`). The [`UiConfig`] fallback defends against a partial deserialize.
/// [`UiConfig`]: crate::agent::config::UiConfig
pub fn resolve_mouse_reporting_toggle(
    effective_config: Option<&TomlValue>,
    ui: &crate::agent::config::UiConfig,
) -> crate::agent::config::Resolved<bool> {
    use crate::agent::config::BoolFlag;
    let from_effective = effective_config
        .and_then(|c| c.get("ui"))
        .and_then(|ui| ui.get("mouse_reporting_toggle"))
        .and_then(|v| v.as_bool());
    BoolFlag::env("GROK_MOUSE_REPORTING_TOGGLE")
        .config(from_effective.or(ui.mouse_reporting_toggle))
        .resolve()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Assumes GROK_MOUSE_REPORTING_TOGGLE is unset in the test env.
    #[test]
    fn resolve_mouse_reporting_toggle_defaults_off() {
        use crate::agent::config::{ConfigSource, UiConfig};
        let resolved = resolve_mouse_reporting_toggle(None, &UiConfig::default());
        assert!(!resolved.value);
        assert_eq!(resolved.source, ConfigSource::Default);
    }

    #[test]
    fn resolve_mouse_reporting_toggle_reads_effective_config() {
        use crate::agent::config::{ConfigSource, UiConfig};
        let effective: TomlValue = toml::from_str("[ui]\nmouse_reporting_toggle = true\n").unwrap();
        let resolved = resolve_mouse_reporting_toggle(Some(&effective), &UiConfig::default());
        assert!(resolved.value);
        assert_eq!(resolved.source, ConfigSource::Config);
    }

    #[test]
    fn resolve_mouse_reporting_toggle_falls_back_to_ui_struct() {
        use crate::agent::config::UiConfig;
        let ui = UiConfig {
            mouse_reporting_toggle: Some(true),
            ..UiConfig::default()
        };
        // With no effective config, the parsed struct field is the fallback layer
        let resolved = resolve_mouse_reporting_toggle(None, &ui);
        assert!(resolved.value);
    }
}

#[cfg(test)]
mod show_thinking_blocks_tests {
    use super::*;
    use crate::agent::config::ConfigSource;

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        let g = super::SHOW_THINKING_BLOCKS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        unsafe { std::env::remove_var(ENV_SHOW_THINKING_BLOCKS) };
        g
    }

    fn toml_ui(v: bool) -> TomlValue {
        toml::from_str(&format!("[ui]\nshow_thinking_blocks = {v}\n")).unwrap()
    }

    fn remote(v: Option<bool>) -> RemoteSettings {
        RemoteSettings {
            show_thinking_blocks: v,
            ..RemoteSettings::default()
        }
    }

    #[test]
    fn defaults_on_when_nothing_set() {
        let _g = guard();
        let r = resolve_show_thinking_blocks(None, None, None, None);
        assert!(r.value, "thinking blocks must default ON");
        assert_eq!(r.source, ConfigSource::Default);
    }

    #[test]
    fn each_layer_can_turn_it_off() {
        let _g = guard();
        let off = toml_ui(false);
        let r = resolve_show_thinking_blocks(Some(&off), None, None, None);
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::Requirement);
        let r = resolve_show_thinking_blocks(None, Some(&off), None, None);
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::Config);
        let r = resolve_show_thinking_blocks(None, None, Some(&off), None);
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::ManagedConfig);
        let r = resolve_show_thinking_blocks(None, None, None, Some(&remote(Some(false))));
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::Remote);
    }

    #[test]
    fn env_overrides_config_and_remote() {
        let _g = guard();
        unsafe { std::env::set_var(ENV_SHOW_THINKING_BLOCKS, "0") };
        let on = toml_ui(true);
        let r = resolve_show_thinking_blocks(None, Some(&on), None, Some(&remote(Some(true))));
        assert!(!r.value, "env must override config + remote");
        assert_eq!(r.source, ConfigSource::Env);
        unsafe { std::env::remove_var(ENV_SHOW_THINKING_BLOCKS) };
    }

    #[test]
    fn requirement_beats_env() {
        let _g = guard();
        unsafe { std::env::set_var(ENV_SHOW_THINKING_BLOCKS, "0") };
        let on = toml_ui(true);
        let r = resolve_show_thinking_blocks(Some(&on), None, None, None);
        assert!(r.value, "requirement must beat env");
        assert_eq!(r.source, ConfigSource::Requirement);
        unsafe { std::env::remove_var(ENV_SHOW_THINKING_BLOCKS) };
    }

    #[test]
    fn config_beats_managed_beats_remote() {
        let _g = guard();
        let off = toml_ui(false);
        let on = toml_ui(true);
        let r =
            resolve_show_thinking_blocks(None, Some(&off), Some(&on), Some(&remote(Some(true))));
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::Config);
        let r = resolve_show_thinking_blocks(None, None, Some(&off), Some(&remote(Some(true))));
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::ManagedConfig);
    }
}

#[cfg(test)]
mod group_tool_verbs_tests {
    use super::*;
    use crate::agent::config::ConfigSource;

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        let g = super::GROUP_TOOL_VERBS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        unsafe { std::env::remove_var(ENV_GROUP_TOOL_VERBS) };
        g
    }

    fn toml_ui(v: bool) -> TomlValue {
        toml::from_str(&format!("[ui]\ngroup_tool_verbs = {v}\n")).unwrap()
    }

    fn remote(v: Option<bool>) -> RemoteSettings {
        RemoteSettings {
            group_tool_verbs: v,
            ..RemoteSettings::default()
        }
    }

    #[test]
    fn defaults_on_when_nothing_set() {
        let _g = guard();
        let r = resolve_group_tool_verbs(None, None, None, None);
        assert!(r.value, "tool-verb grouping must default ON");
        assert_eq!(r.source, ConfigSource::Default);
    }

    #[test]
    fn each_layer_can_turn_it_off() {
        let _g = guard();
        let off = toml_ui(false);
        let r = resolve_group_tool_verbs(Some(&off), None, None, None);
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::Requirement);
        unsafe { std::env::set_var(ENV_GROUP_TOOL_VERBS, "0") };
        let r = resolve_group_tool_verbs(None, None, None, None);
        assert!(!r.value, "env disable must beat the true default");
        assert_eq!(r.source, ConfigSource::Env);
        unsafe { std::env::remove_var(ENV_GROUP_TOOL_VERBS) };
        let r = resolve_group_tool_verbs(None, Some(&off), None, None);
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::Config);
        let r = resolve_group_tool_verbs(None, None, Some(&off), None);
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::ManagedConfig);
        let r = resolve_group_tool_verbs(None, None, None, Some(&remote(Some(false))));
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::Remote);
    }

    #[test]
    fn env_overrides_config_and_remote() {
        let _g = guard();
        unsafe { std::env::set_var(ENV_GROUP_TOOL_VERBS, "0") };
        let on = toml_ui(true);
        let r = resolve_group_tool_verbs(None, Some(&on), None, Some(&remote(Some(true))));
        assert!(!r.value, "env must override config + remote");
        assert_eq!(r.source, ConfigSource::Env);
        unsafe { std::env::remove_var(ENV_GROUP_TOOL_VERBS) };
    }

    #[test]
    fn requirement_beats_env() {
        let _g = guard();
        unsafe { std::env::set_var(ENV_GROUP_TOOL_VERBS, "0") };
        let on = toml_ui(true);
        let r = resolve_group_tool_verbs(Some(&on), None, None, None);
        assert!(r.value, "requirement must beat env");
        assert_eq!(r.source, ConfigSource::Requirement);
        unsafe { std::env::remove_var(ENV_GROUP_TOOL_VERBS) };
    }

    #[test]
    fn config_beats_managed_beats_remote() {
        let _g = guard();
        let off = toml_ui(false);
        let on = toml_ui(true);
        let r = resolve_group_tool_verbs(None, Some(&off), Some(&on), Some(&remote(Some(true))));
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::Config);
        let r = resolve_group_tool_verbs(None, None, Some(&off), Some(&remote(Some(true))));
        assert!(!r.value);
        assert_eq!(r.source, ConfigSource::ManagedConfig);
    }
}

#[cfg(test)]
mod collapsed_edit_blocks_tests {
    use super::*;
    use crate::agent::config::ConfigSource;

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        let g = super::COLLAPSED_EDIT_BLOCKS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        unsafe { std::env::remove_var(ENV_COLLAPSED_EDIT_BLOCKS) };
        g
    }

    fn toml_ui(v: bool) -> TomlValue {
        toml::from_str(&format!("[ui]\ncollapsed_edit_blocks = {v}\n")).unwrap()
    }

    fn remote(v: Option<bool>) -> RemoteSettings {
        RemoteSettings {
            collapsed_edit_blocks: v,
            ..RemoteSettings::default()
        }
    }

    #[test]
    fn defaults_off_when_nothing_set() {
        let _g = guard();
        let r = resolve_collapsed_edit_blocks(None, None, None, None);
        assert!(!r.value, "collapsed edit blocks must default OFF");
        assert_eq!(r.source, ConfigSource::Default);
    }

    #[test]
    fn each_layer_can_turn_it_on() {
        let _g = guard();
        let on = toml_ui(true);
        let r = resolve_collapsed_edit_blocks(Some(&on), None, None, None);
        assert!(r.value);
        assert_eq!(r.source, ConfigSource::Requirement);
        unsafe { std::env::set_var(ENV_COLLAPSED_EDIT_BLOCKS, "1") };
        let r = resolve_collapsed_edit_blocks(None, None, None, None);
        assert!(r.value, "env enable must beat the false default");
        assert_eq!(r.source, ConfigSource::Env);
        unsafe { std::env::remove_var(ENV_COLLAPSED_EDIT_BLOCKS) };
        let r = resolve_collapsed_edit_blocks(None, Some(&on), None, None);
        assert!(r.value);
        assert_eq!(r.source, ConfigSource::Config);
        let r = resolve_collapsed_edit_blocks(None, None, Some(&on), None);
        assert!(r.value);
        assert_eq!(r.source, ConfigSource::ManagedConfig);
        let r = resolve_collapsed_edit_blocks(None, None, None, Some(&remote(Some(true))));
        assert!(r.value);
        assert_eq!(r.source, ConfigSource::Remote);
    }

    #[test]
    fn env_overrides_config_and_remote() {
        let _g = guard();
        unsafe { std::env::set_var(ENV_COLLAPSED_EDIT_BLOCKS, "0") };
        let on = toml_ui(true);
        let r = resolve_collapsed_edit_blocks(None, Some(&on), None, Some(&remote(Some(true))));
        assert!(!r.value, "env must override config + remote");
        assert_eq!(r.source, ConfigSource::Env);
        unsafe { std::env::remove_var(ENV_COLLAPSED_EDIT_BLOCKS) };
    }

    #[test]
    fn requirement_beats_env() {
        let _g = guard();
        unsafe { std::env::set_var(ENV_COLLAPSED_EDIT_BLOCKS, "1") };
        let off = toml_ui(false);
        let r = resolve_collapsed_edit_blocks(Some(&off), None, None, None);
        assert!(!r.value, "requirement must beat env");
        assert_eq!(r.source, ConfigSource::Requirement);
        unsafe { std::env::remove_var(ENV_COLLAPSED_EDIT_BLOCKS) };
    }

    #[test]
    fn config_beats_managed_beats_remote() {
        let _g = guard();
        let off = toml_ui(false);
        let on = toml_ui(true);
        let r =
            resolve_collapsed_edit_blocks(None, Some(&on), Some(&off), Some(&remote(Some(false))));
        assert!(r.value);
        assert_eq!(r.source, ConfigSource::Config);
        let r = resolve_collapsed_edit_blocks(None, None, Some(&on), Some(&remote(Some(false))));
        assert!(r.value);
        assert_eq!(r.source, ConfigSource::ManagedConfig);
    }
}

#[cfg(test)]
mod density_tier_tests {
    use super::{DensityTier, combine_density_tiers, resolve_density_fold};
    use crate::agent::config::ToolCallDensity;

    fn tier(
        density: Option<ToolCallDensity>,
        group: Option<bool>,
        collapsed: Option<bool>,
    ) -> DensityTier {
        DensityTier {
            density,
            group_tool_verbs: group,
            collapsed_edit_blocks: collapsed,
        }
    }

    #[test]
    fn higher_density_ignores_a_lower_legacy_flag() {
        let fold = combine_density_tiers(&[
            tier(Some(ToolCallDensity::Detailed), None, None),
            tier(None, Some(false), Some(true)),
        ]);
        assert_eq!(fold.density, ToolCallDensity::Detailed);
        assert!(fold.group_tool_verbs);
        assert!(!fold.collapsed_edit_blocks);
    }

    #[test]
    fn higher_legacy_flag_still_changes_a_lower_density() {
        let fold = combine_density_tiers(&[
            tier(None, Some(false), Some(true)),
            tier(Some(ToolCallDensity::Detailed), None, None),
        ]);
        assert_eq!(fold.density, ToolCallDensity::Detailed);
        assert!(!fold.group_tool_verbs);
        assert!(fold.collapsed_edit_blocks);
    }

    #[test]
    fn same_tier_legacy_flag_still_applies() {
        let fold = combine_density_tiers(&[tier(
            Some(ToolCallDensity::CompactGrouped),
            Some(false),
            Some(true),
        )]);
        assert_eq!(fold.density, ToolCallDensity::CompactGrouped);
        assert!(!fold.group_tool_verbs);
        assert!(fold.collapsed_edit_blocks);
    }

    #[test]
    fn nothing_set_uses_the_defaults() {
        let fold = combine_density_tiers(&[DensityTier::default()]);
        assert_eq!(fold.density, ToolCallDensity::CompactAllGrouped);
        assert!(fold.group_tool_verbs);
        assert!(!fold.collapsed_edit_blocks);
    }

    #[test]
    fn user_density_beats_managed_legacy_flags() {
        let _group = super::GROUP_TOOL_VERBS_ENV_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let _collapsed = super::COLLAPSED_EDIT_BLOCKS_ENV_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        unsafe {
            std::env::remove_var(super::ENV_GROUP_TOOL_VERBS);
            std::env::remove_var(super::ENV_COLLAPSED_EDIT_BLOCKS);
            std::env::remove_var(super::ENV_TOOL_CALL_DENSITY);
        }
        let user = toml::from_str("[ui]\ntool_call_density = \"detailed\"\n").unwrap();
        let managed = toml::from_str(
            "[ui]\ngroup_tool_verbs = false\ncollapsed_edit_blocks = true\n",
        )
        .unwrap();
        let fold = resolve_density_fold(None, Some(&user), Some(&managed), None);
        assert_eq!(fold.density, ToolCallDensity::Detailed);
        assert!(fold.group_tool_verbs);
        assert!(!fold.collapsed_edit_blocks);
    }
}
