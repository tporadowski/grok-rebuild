use std::path::{Path, PathBuf};

/// Path to the legacy project-hook trust file (`<user_grok_home>/trusted-hook-projects`), or `None` when no user grok home resolves.
/// It is retained only for the one-time migration into folder-trust.
pub fn legacy_trust_file_path() -> Option<PathBuf> {
    Some(xai_grok_config::user_grok_home()?.join(xai_grok_config::TRUSTED_HOOK_PROJECTS_FILENAME))
}

/// The legacy format is one canonical absolute path per line; blank and `#`-comment lines are skipped.
/// Any other read error is returned as `Err` so the caller does not mistake an unreadable file for an empty one and consume it.
/// The one-time migration that seeds folder-trust from prior grants consumes this list.
pub fn list_trusted_projects_with_file(trust_file: &Path) -> std::io::Result<Vec<PathBuf>> {
    let content = match std::fs::read_to_string(trust_file) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(PathBuf::from)
        .collect())
}

/// Whether the user trusts a workspace, which decides if its project hooks and plugins load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Trust {
    Trusted,
    Untrusted,
}

impl Trust {
    #[must_use]
    pub fn from_verdict(is_trusted: bool) -> Trust {
        if is_trusted {
            Trust::Trusted
        } else {
            Trust::Untrusted
        }
    }

    #[must_use]
    pub fn allows_project_sources(self) -> bool {
        matches!(self, Trust::Trusted)
    }
}

// ── Hook enable/disable ─────────────────────────────────────────────────

/// Why a hook is skipped at dispatch and shown disabled in the modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookSkipReason {
    /// Its `enabled` flag is off or its name is in `$GROK_HOME/disabled-hooks`.
    UserDisabled,
    /// `allow_managed_hooks_only` is pinned and the hook is not managed policy.
    ManagedOnly,
}

/// One-shot snapshot of the per-spec skip inputs: the disabled-hooks file and the `allow_managed_hooks_only` pin.
/// [`Self::skip_reason`] is the one rule the dispatcher, the stop-gate guard, and the modal apply, so they cannot disagree about what runs.
#[derive(Debug, Default)]
pub struct DisabledHooks {
    names: std::collections::HashSet<String>,
    managed_only: bool,
    /// The disabled-hooks file exists but could not be read. Non-managed hooks stay off.
    unreadable: bool,
}

impl DisabledHooks {
    pub fn new<I: IntoIterator<Item = String>>(names: I, managed_only: bool) -> Self {
        Self {
            names: names.into_iter().collect(),
            managed_only,
            unreadable: false,
        }
    }

    /// Read the disabled-hooks file under `grok_home`; `managed_only` is the resolved `allow_managed_hooks_only` pin, which the caller reads from managed settings.
    /// A missing file disables nothing. A file that exists but cannot be read keeps every non-managed hook disabled.
    pub fn load(grok_home: Option<&Path>, managed_only: bool) -> Self {
        let Some(grok_home) = grok_home else {
            return Self::new([], managed_only);
        };
        match std::fs::read_to_string(grok_home.join(DISABLED_HOOKS_FILE)) {
            Ok(content) => Self::new(
                content
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .map(str::to_string),
                managed_only,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Self::new([], managed_only)
            }
            Err(_) => Self {
                names: std::collections::HashSet::new(),
                managed_only,
                unreadable: true,
            },
        }
    }

    pub fn contains(&self, hook_name: &str) -> bool {
        self.names.contains(hook_name)
    }

    pub fn managed_only(&self) -> bool {
        self.managed_only
    }

    /// Managed-policy hooks are never skipped; the lockdown outranks a user disable as the reported reason.
    pub fn skip_reason(&self, spec: &crate::config::HookSpec) -> Option<HookSkipReason> {
        if spec.is_managed_policy() {
            return None;
        }
        if self.managed_only {
            return Some(HookSkipReason::ManagedOnly);
        }
        if self.unreadable || !spec.enabled || self.names.contains(&spec.name) {
            return Some(HookSkipReason::UserDisabled);
        }
        None
    }

    /// Whether dispatch skips `spec`; also what the modal shows as disabled.
    pub fn blocks(&self, spec: &crate::config::HookSpec) -> bool {
        self.skip_reason(spec).is_some()
    }
}

fn is_hook_disabled_with_file(hook_name: &str, file: &Path) -> bool {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content
        .lines()
        .any(|l| !l.trim().is_empty() && !l.trim().starts_with('#') && l.trim() == hook_name)
}

/// Disable a hook by name (append to `$GROK_HOME/disabled-hooks`).
pub fn disable_hook(hook_name: &str) -> Result<(), String> {
    let file = disabled_hooks_file_path()
        .ok_or_else(|| "no user grok home (set $GROK_HOME or $HOME)".to_string())?;
    disable_hook_with_file(hook_name, &file)
}

fn disable_hook_with_file(hook_name: &str, file: &Path) -> Result<(), String> {
    if is_hook_disabled_with_file(hook_name, file) {
        return Ok(());
    }
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .map_err(|e| format!("failed to open disabled-hooks file: {e}"))?;
    writeln!(f, "{hook_name}").map_err(|e| format!("failed to write disabled-hooks file: {e}"))?;
    Ok(())
}

/// Enable a hook by name (remove from `$GROK_HOME/disabled-hooks`).
pub fn enable_hook(hook_name: &str) -> Result<bool, String> {
    match disabled_hooks_file_path() {
        Some(file) => enable_hook_with_file(hook_name, &file),
        None => Ok(false),
    }
}

fn enable_hook_with_file(hook_name: &str, file: &Path) -> Result<bool, String> {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("failed to read disabled-hooks file: {e}")),
    };
    let mut found = false;
    let new_lines: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed == hook_name {
                found = true;
                false
            } else {
                true
            }
        })
        .collect();
    if !found {
        return Ok(false);
    }
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    let mut f = std::fs::File::create(file)
        .map_err(|e| format!("failed to open disabled-hooks file: {e}"))?;
    for line in new_lines {
        writeln!(f, "{line}").map_err(|e| format!("failed to write disabled-hooks file: {e}"))?;
    }
    Ok(true)
}

const DISABLED_HOOKS_FILE: &str = "disabled-hooks";

/// Returns the path to `$GROK_HOME/disabled-hooks`, or `None` when no user grok home resolves.
fn disabled_hooks_file_path() -> Option<PathBuf> {
    Some(xai_grok_config::user_grok_home()?.join(DISABLED_HOOKS_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn unreadable_disabled_hooks_file_keeps_user_hooks_off() {
        let dir = std::env::temp_dir().join(format!(
            "grok-disabled-hooks-unreadable-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(DISABLED_HOOKS_FILE);
        std::fs::write(&file, "kept-off\n").unwrap();
        let mut perms = std::fs::metadata(&file).unwrap().permissions();
        perms.set_mode(0);
        std::fs::set_permissions(&file, perms).unwrap();
        let loaded = DisabledHooks::load(Some(&dir), false);
        let mut restore = std::fs::metadata(&file).unwrap().permissions();
        restore.set_mode(0o644);
        std::fs::set_permissions(&file, restore).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let user = spec_named("other");
        let mut policy = spec_named("policy");
        policy.layer = crate::config::HookProvenance::SystemManaged;
        assert_eq!(
            loaded.skip_reason(&user),
            Some(HookSkipReason::UserDisabled)
        );
        assert_eq!(loaded.skip_reason(&policy), None);
    }

    fn spec_named(name: &str) -> crate::config::HookSpec {
        crate::config::HookSpec {
            name: name.to_string(),
            event: crate::event::HookEventName::PreToolUse,
            handler_type: crate::config::HandlerType::Command,
            configured_matcher: None,
            matcher: None,
            enabled: true,
            command: Some(PathBuf::from("guard")),
            command_raw: Some("guard".to_string()),
            url: None,
            url_raw: None,
            timeout_ms: 1000,
            source_dir: PathBuf::from("/tmp"),
            extra_env: std::collections::HashMap::new(),
            args: Vec::new(),
            fail_closed: false,
            layer: crate::config::HookProvenance::File,
        }
    }

    /// Each test creates its own legacy file in its own temp dir, so no state is shared.
    fn trust_file_in(dir: &Path) -> PathBuf {
        let grok_dir = dir.join(".grok");
        std::fs::create_dir_all(&grok_dir).unwrap();
        grok_dir.join("trusted-hook-projects")
    }

    #[test]
    fn list_trusted_projects_parses_paths_skipping_comments_and_blanks() {
        let home = tempfile::tempdir().unwrap();
        let trust_file = trust_file_in(home.path());
        std::fs::write(
            &trust_file,
            "# comment\n\n/abs/project/one\n  /abs/project/two  \n# trailing\n",
        )
        .unwrap();

        let projects = list_trusted_projects_with_file(&trust_file).unwrap();
        assert_eq!(
            projects,
            vec![
                PathBuf::from("/abs/project/one"),
                PathBuf::from("/abs/project/two"),
            ]
        );
    }

    #[test]
    fn list_trusted_projects_missing_file_is_empty() {
        // The migration treats a missing file as "nothing to migrate", not as an unreadable file
        let projects =
            list_trusted_projects_with_file(Path::new("/nonexistent/trusted-hook-projects"))
                .expect("missing file resolves to Ok(empty)");
        assert!(projects.is_empty());
    }
}
