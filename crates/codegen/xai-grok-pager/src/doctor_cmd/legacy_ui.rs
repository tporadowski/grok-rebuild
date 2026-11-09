//! Leftover UI keys. `[ui] vim_mode` is the old bool for `[keys] vim`.
//! `[ui] simple_mode` is retired. Neither stops startup.

use std::path::Path;

use crate::diagnostics::{
    CONFIG_LEGACY_UI_ID, DiagnosticFinding, DiagnosticReport, FindingDisposition,
};
use xai_grok_config::USER_CONFIG_FILENAME;

pub(crate) fn append_legacy_ui_findings(report: &mut DiagnosticReport) {
    report.findings.extend(legacy_ui_findings_at(
        xai_grok_config::system_config_dir().as_deref(),
        xai_dirs::resolve_grok_home().as_deref(),
    ));
}

/// Findings for an explicit system directory and user home. `None` skips that side.
pub(super) fn legacy_ui_findings_at(
    system_dir: Option<&Path>,
    user_home: Option<&Path>,
) -> Vec<DiagnosticFinding> {
    let mut findings = Vec::new();
    if let Some(dir) = system_dir {
        findings.extend(findings_in_file(&dir.join(USER_CONFIG_FILENAME)));
    }
    if let Some(home) = user_home {
        findings.extend(findings_in_file(&home.join(USER_CONFIG_FILENAME)));
    }
    findings
}

fn findings_in_file(path: &Path) -> Vec<DiagnosticFinding> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    // `str::parse::<toml::Value>()` reads one value, so a `[ui]` table is rejected.
    let Ok(value) = toml::from_str::<toml::Value>(&raw) else {
        return Vec::new();
    };
    let Some(ui) = value.get("ui").and_then(toml::Value::as_table) else {
        return Vec::new();
    };
    let shown = path.display().to_string();
    let mut findings = Vec::new();
    if ui.contains_key("vim_mode") {
        findings.push(legacy_finding(format!(
            "{shown}: [ui] vim_mode is leftover. Set [keys] vim instead (off, nav, or all)."
        )));
    }
    if ui.contains_key("simple_mode") {
        findings.push(legacy_finding(format!(
            "{shown}: [ui] simple_mode is retired. Remove that key."
        )));
    }
    findings
}

fn legacy_finding(message: String) -> DiagnosticFinding {
    DiagnosticFinding {
        id: CONFIG_LEGACY_UI_ID,
        disposition: FindingDisposition::Recommendation,
        message,
        remediation: None,
        automatic_remediation: None,
        note: None,
    }
}

#[cfg(test)]
mod tests {
    use super::legacy_ui_findings_at;

    #[test]
    fn leftover_vim_mode_and_retired_simple_mode_are_named() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        std::fs::write(&path, "[ui]\nvim_mode = true\nsimple_mode = false\n").unwrap();
        let findings = legacy_ui_findings_at(None, Some(dir.path()));
        let text = findings
            .iter()
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            text.contains(&path.display().to_string()),
            "findings {}: {text}",
            findings.len()
        );
        assert!(text.contains("[ui] vim_mode"), "{text}");
        assert!(text.contains("[keys] vim"), "{text}");
        assert!(text.contains("[ui] simple_mode is retired"), "{text}");
        assert!(
            findings
                .iter()
                .all(|finding| finding.id.to_string() == "config.legacy-ui"),
            "{text}"
        );
    }

    #[test]
    fn a_config_without_those_keys_is_quiet() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"), "[ui]\ncompact_mode = true\n").unwrap();
        assert!(legacy_ui_findings_at(None, Some(dir.path())).is_empty());
    }
}
