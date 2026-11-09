//! Config that stops Grok from starting. `grok doctor` runs before that check,
//! so a broken file used to leave the report at "No issues found."

use crate::diagnostics::{
    CONFIG_STARTUP_ID, DiagnosticFinding, DiagnosticReport, FindingDisposition,
};
use xai_grok_config::{
    MANAGED_CONFIG_FILENAME, REQUIREMENTS_FILENAME, StartupConfigBlocker, USER_CONFIG_FILENAME,
    load_config_file, startup_config_blockers_at,
};
use xai_grok_shell::util::config::{VersionBoundSource, required_version_startup_block};

pub(super) fn append_startup_config_findings(report: &mut DiagnosticReport) {
    report.findings.extend(startup_config_findings());
}

fn startup_config_findings() -> Vec<DiagnosticFinding> {
    let blockers = startup_config_blockers_at(
        xai_grok_config::system_config_dir().as_deref(),
        xai_dirs::resolve_grok_home().as_deref(),
    );
    let mut findings = findings_from_blockers(&blockers);
    if blockers.is_empty()
        && let Some(message) = live_typed_block()
    {
        findings.push(startup_finding(message));
    }
    if let Some(message) = live_version_block() {
        findings.push(startup_finding(message));
    }
    findings
}

pub(super) fn findings_from_blockers(
    blockers: &[StartupConfigBlocker],
) -> Vec<DiagnosticFinding> {
    blockers
        .iter()
        .map(|blocker| startup_finding(blocker_message(blocker)))
        .collect()
}

pub(super) fn startup_finding(message: String) -> DiagnosticFinding {
    DiagnosticFinding {
        id: CONFIG_STARTUP_ID,
        disposition: FindingDisposition::Issue,
        message,
        remediation: None,
        automatic_remediation: None,
        note: Some("Grok will not start until this config is fixed.".to_owned()),
    }
}

fn blocker_message(blocker: &StartupConfigBlocker) -> String {
    let path = blocker.path.display().to_string();
    if blocker.detail.contains(&path) {
        blocker.detail.clone()
    } else {
        format!("{path}: {}", blocker.detail)
    }
}

pub(super) fn typed_startup_message(
    raw: &toml::Value,
    files: &[(String, toml::Value)],
) -> Option<String> {
    let err = match xai_grok_shell::agent::config::Config::new_from_toml_cfg(raw) {
        Err(err) => err,
        Ok(cfg) => cfg.startup_refusal().err()?,
    };
    let paths = files_for_refusal(&err, files);
    if paths.is_empty() {
        Some(err)
    } else {
        Some(format!("{err} ({})", paths.join(", ")))
    }
}

fn files_for_refusal(err: &str, files: &[(String, toml::Value)]) -> Vec<String> {
    files
        .iter()
        .filter(|(_, value)| value_matches_refusal(value, err))
        .map(|(path, _)| path.clone())
        .collect()
}

fn value_matches_refusal(value: &toml::Value, err: &str) -> bool {
    if err.contains("[shell_environment_policy]") {
        return value.get("shell_environment_policy").is_some();
    }
    for key in ["allowed_models", "disabled_models", "hidden_models"] {
        if err.contains(key) {
            return value.get("models").and_then(|models| models.get(key)).is_some();
        }
    }
    if err.contains("mtls_cert_dir") {
        return value
            .get("model")
            .and_then(toml::Value::as_table)
            .is_some_and(|models| {
                models
                    .values()
                    .any(|model| model.get("mtls_cert_dir").is_some())
            });
    }
    false
}

fn live_typed_block() -> Option<String> {
    let files = parsed_startup_files();
    match xai_grok_config::effective_config::load_effective_config() {
        Err(err) => Some(format!("Failed to load config: {err}")),
        Ok(raw) => typed_startup_message(&raw, &files),
    }
}

fn live_version_block() -> Option<String> {
    let current = xai_grok_version::installed_semver().ok()?;
    let files = parsed_startup_files();
    let mut minimums = bounds_from_files(&files, "required_minimum_version");
    let mut maximums = bounds_from_files(&files, "required_maximum_version");
    if let Ok(raw) = std::env::var("GROK_REQUIRED_MINIMUM_VERSION") {
        minimums.push(("$GROK_REQUIRED_MINIMUM_VERSION".to_owned(), raw));
    }
    if let Ok(raw) = std::env::var("GROK_REQUIRED_MAXIMUM_VERSION") {
        maximums.push(("$GROK_REQUIRED_MAXIMUM_VERSION".to_owned(), raw));
    }
    let minimum_sources: Vec<VersionBoundSource<'_>> = minimums
        .iter()
        .map(|(location, raw)| VersionBoundSource { location, raw })
        .collect();
    let maximum_sources: Vec<VersionBoundSource<'_>> = maximums
        .iter()
        .map(|(location, raw)| VersionBoundSource { location, raw })
        .collect();
    required_version_startup_block(&current, &minimum_sources, &maximum_sources)
}

fn bounds_from_files(files: &[(String, toml::Value)], key: &str) -> Vec<(String, String)> {
    files
        .iter()
        .filter_map(|(path, value)| {
            let raw = value.get("cli")?.get(key)?.as_str()?;
            Some((path.clone(), raw.to_owned()))
        })
        .collect()
}

fn parsed_startup_files() -> Vec<(String, toml::Value)> {
    let mut files = Vec::new();
    let mut push = |path: std::path::PathBuf| {
        if path.is_file()
            && let Ok(value) = load_config_file(&path)
        {
            files.push((path.display().to_string(), value));
        }
    };
    if let Some(dir) = xai_grok_config::system_config_dir() {
        push(dir.join(MANAGED_CONFIG_FILENAME));
        push(dir.join(REQUIREMENTS_FILENAME));
    }
    if let Some(home) = xai_dirs::resolve_grok_home() {
        push(home.join(MANAGED_CONFIG_FILENAME));
        push(home.join(USER_CONFIG_FILENAME));
        push(home.join(REQUIREMENTS_FILENAME));
    }
    files
}
