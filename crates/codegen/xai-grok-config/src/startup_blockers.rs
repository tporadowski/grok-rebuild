//! Config files and requirements that stop Grok before the session starts.
//!
//! `ConfigLayers::load` fails closed on `config.toml` and `managed_config.toml`.
//! [`crate::validate_requirements`] fails closed on a `fail_closed` requirements
//! file whose `[[version_overrides]]` this build cannot apply. A syntax error in
//! `requirements.toml` does not, because that loader skips the layer.

use std::path::{Path, PathBuf};

use crate::loader::{
    MANAGED_CONFIG_FILENAME, REQUIREMENTS_FILENAME, USER_CONFIG_FILENAME, load_config_file,
};
use crate::validation::{mdm_requirements_startup_error, validate_requirements_layer};

/// One file (or the MDM requirements label) that stops startup, with the loader's detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupConfigBlocker {
    pub path: PathBuf,
    pub detail: String,
}

/// Blockers under an explicit system directory and user home.
/// `None` skips that side. Missing files are not blockers.
pub fn startup_config_blockers_at(
    system_dir: Option<&Path>,
    user_home: Option<&Path>,
) -> Vec<StartupConfigBlocker> {
    let mut out = Vec::new();
    if let Some(dir) = system_dir {
        push_config_file(&mut out, &dir.join(MANAGED_CONFIG_FILENAME));
        push_requirements(&mut out, &dir.join(REQUIREMENTS_FILENAME));
    }
    if let Some(home) = user_home {
        push_config_file(&mut out, &home.join(MANAGED_CONFIG_FILENAME));
        push_config_file(&mut out, &home.join(USER_CONFIG_FILENAME));
        push_requirements(&mut out, &home.join(REQUIREMENTS_FILENAME));
    }
    if let Some(detail) = mdm_requirements_startup_error() {
        out.push(StartupConfigBlocker {
            path: PathBuf::from(crate::macos_managed::MDM_REQUIREMENTS_SOURCE),
            detail,
        });
    }
    out
}

fn push_config_file(out: &mut Vec<StartupConfigBlocker>, path: &Path) {
    if !path.is_file() {
        return;
    }
    if let Err(err) = load_config_file(path) {
        out.push(StartupConfigBlocker {
            path: path.to_path_buf(),
            detail: err.to_string(),
        });
    }
}

fn push_requirements(out: &mut Vec<StartupConfigBlocker>, path: &Path) {
    if let Err(err) = validate_requirements_layer(path) {
        out.push(StartupConfigBlocker {
            path: path.to_path_buf(),
            detail: err.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broken_config_toml_names_that_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(USER_CONFIG_FILENAME);
        std::fs::write(&path, "not = [valid\n").unwrap();

        let blockers = startup_config_blockers_at(None, Some(dir.path()));

        let blocker = blockers.first().expect("the broken file blocks startup");
        assert_eq!(blockers.len(), 1);
        assert_eq!(blocker.path, path);
        assert!(
            blocker.detail.contains("TOML parse error at line 1"),
            "{}",
            blocker.detail
        );
    }

    #[test]
    fn an_empty_home_is_not_a_startup_blocker() {
        let dir = tempfile::tempdir().unwrap();
        assert!(startup_config_blockers_at(None, Some(dir.path())).is_empty());
    }

    #[test]
    fn fail_closed_requirements_name_the_file_and_the_setting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(REQUIREMENTS_FILENAME);
        std::fs::write(
            &path,
            "fail_closed = true\n[[version_overrides]]\nminimum_version = \"not-a-version\"\n",
        )
        .unwrap();

        let blockers = startup_config_blockers_at(None, Some(dir.path()));

        let blocker = blockers
            .first()
            .expect("fail_closed requirements block startup");
        assert_eq!(blockers.len(), 1);
        assert_eq!(blocker.path, path);
        let detail = &blocker.detail;
        assert!(detail.contains(&path.display().to_string()), "{detail}");
        assert!(detail.contains("version_overrides"), "{detail}");
        assert!(detail.contains("fail_closed"), "{detail}");
    }
}
