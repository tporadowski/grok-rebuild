//! `--add-dir`: extra workspace folders, refused unless the folder is already trusted.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use agent_client_protocol as acp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddDirTrust {
    Trusted,
    Untrusted,
}

static STARTUP_ADD_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();

pub fn set_startup_add_dirs(dirs: Vec<PathBuf>) {
    let _ = STARTUP_ADD_DIRS.set(dirs);
}

pub fn startup_add_dirs() -> &'static [PathBuf] {
    STARTUP_ADD_DIRS.get().map(Vec::as_slice).unwrap_or(&[])
}

pub fn attach_startup_dirs(req: acp::NewSessionRequest) -> acp::NewSessionRequest {
    let dirs = startup_add_dirs();
    if dirs.is_empty() {
        req
    } else {
        req.additional_directories(dirs.to_vec())
    }
}

/// Keep directories that exist and pass `trust`. The first failure is the whole error.
pub fn refuse_untrusted_add_dirs(
    dirs: &[PathBuf],
    trust: impl Fn(&Path) -> AddDirTrust,
) -> Result<Vec<PathBuf>, String> {
    let mut accepted = Vec::with_capacity(dirs.len());
    let mut bad = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            bad.push(format!("--add-dir {}: not a directory", dir.display()));
            continue;
        }
        match trust(dir) {
            AddDirTrust::Trusted => accepted.push(dir.clone()),
            AddDirTrust::Untrusted => bad.push(format!(
                "--add-dir {}: folder is not trusted. Trust it before adding it as a workspace folder.",
                dir.display()
            )),
        }
    }
    if bad.is_empty() {
        Ok(accepted)
    } else {
        Err(bad.join("\n"))
    }
}

pub fn directory_trust(dir: &Path) -> AddDirTrust {
    if xai_grok_workspace::folder_trust::add_directory_trusted(dir) {
        AddDirTrust::Trusted
    } else {
        AddDirTrust::Untrusted
    }
}

#[cfg(test)]
mod tests {
    use super::{AddDirTrust, refuse_untrusted_add_dirs};

    #[test]
    fn untrusted_directory_is_refused_and_a_trusted_one_is_kept() {
        let dir = std::env::temp_dir();
        let err = refuse_untrusted_add_dirs(&[dir.clone()], |_| AddDirTrust::Untrusted)
            .expect_err("untrusted");
        assert!(err.contains("not trusted"), "{err}");
        let ok = refuse_untrusted_add_dirs(&[dir.clone()], |_| AddDirTrust::Trusted).unwrap();
        assert_eq!(ok, vec![dir]);
    }

    #[test]
    fn missing_directory_is_refused() {
        let missing = std::env::temp_dir().join("grok-add-dir-missing-does-not-exist");
        let err = refuse_untrusted_add_dirs(&[missing], |_| AddDirTrust::Trusted).unwrap_err();
        assert!(err.contains("not a directory"), "{err}");
    }
}
