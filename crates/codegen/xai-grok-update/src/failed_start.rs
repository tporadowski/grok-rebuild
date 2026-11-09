//! A version whose binary failed the startup smoke test.
//!
//! Automatic updates do not download that version again. A newer published
//! version is still installed. Each launch tells the user to run `grok update`.

use std::path::{Path, PathBuf};

const FILE_NAME: &str = "failed_start_version";

pub fn failed_start_notice(version: &str) -> String {
    format!(
        "Version {version} failed to start on this machine. Run `grok update` to see why."
    )
}

/// `Some` when `target` is the version that already failed to start.
/// A different version, including a newer one, is not held.
pub fn hold_failed_start(target: &str, failed: Option<&str>) -> Option<String> {
    let failed = failed?;
    if versions_match(target, failed) {
        Some(failed_start_notice(failed))
    } else {
        None
    }
}

pub fn record(version: &str) {
    let _ = write_at(&record_path(), version);
}

pub fn clear_recorded() {
    let _ = std::fs::remove_file(record_path());
}

pub fn recorded() -> Option<String> {
    read_at(&record_path())
}

pub fn hold_notice(target: &str) -> Option<String> {
    hold_failed_start(target, recorded().as_deref())
}

fn record_path() -> PathBuf {
    xai_grok_shell::util::grok_home::grok_home().join(FILE_NAME)
}

pub fn write_at(path: &Path, version: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, version.trim())
}

pub fn read_at(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let version = raw.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

fn versions_match(left: &str, right: &str) -> bool {
    let left = normalize(left);
    let right = normalize(right);
    match (semver::Version::parse(left), semver::Version::parse(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn normalize(version: &str) -> &str {
    version.trim().strip_prefix('v').unwrap_or(version.trim())
}

#[cfg(test)]
mod tests {
    use super::{failed_start_notice, hold_failed_start, read_at, write_at};

    #[test]
    fn the_same_failed_version_is_not_retried() {
        let notice = hold_failed_start("1.0.73", Some("1.0.73")).expect("hold");
        assert_eq!(notice, failed_start_notice("1.0.73"));
        assert!(notice.contains("grok update"), "{notice}");
        assert_eq!(
            hold_failed_start("v1.0.73", Some("1.0.73")).as_deref(),
            Some(notice.as_str())
        );
    }

    #[test]
    fn a_newer_version_is_not_held() {
        assert!(hold_failed_start("1.0.74", Some("1.0.73")).is_none());
        assert!(hold_failed_start("1.0.72", Some("1.0.73")).is_none());
        assert!(hold_failed_start("1.0.73", None).is_none());
    }

    #[test]
    fn the_failed_version_round_trips_through_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("failed_start_version");
        write_at(&path, " 1.0.73\n").unwrap();
        assert_eq!(read_at(&path).as_deref(), Some("1.0.73"));
    }
}
