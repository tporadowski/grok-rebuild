//! Normalization of `read_only` / `read_write` sandbox config entries.
//!
//! This security-sensitive parser is kept separate from the profile resolver so the policy it implements stays small and easy to audit.
//! Allow paths are literal directory grants.
//! `~` and `~/...` expand to the user home. The other rewrite is stripping one trailing recursive glob down to the directory the user plainly meant.
//! Everything else either passes through byte-for-byte or is rejected, never widened.

use std::path::{Path, PathBuf};

use crate::deny::is_glob;

/// Allow paths are literal directory grants. `~` and `~/...` expand to the user home before anything else.
/// One trailing `/**`, `/**/`, `/**/*`, or `/*` is stripped to the parent
/// (root forms grant `/`); a literal `**` directory used to hide the intended tree. Whitespace is not trimmed, and any
/// entry still glob-shaped after that strip is skipped rather than widened. `deny` globs are unchanged.
pub(crate) fn normalize_allow_path(raw: &str) -> Option<PathBuf> {
    normalize_allow_path_with_home(raw, xai_dirs::home_dir().as_deref())
}

pub(crate) fn normalize_allow_path_with_home(raw: &str, home: Option<&Path>) -> Option<PathBuf> {
    if raw.is_empty() {
        return None;
    }
    if raw.trim() != raw {
        tracing::warn!(
            path = %raw,
            "sandbox allow path has surrounding whitespace; whitespace is significant in \
             literal paths, so fix the entry; skipping"
        );
        return None;
    }

    let expanded = expand_user_home(raw, home)?;
    let mut s = expanded.as_str();
    if let Some(parent) = s
        .strip_suffix("/**/*")
        .or_else(|| s.strip_suffix("/**/"))
        .or_else(|| s.strip_suffix("/**"))
        .or_else(|| s.strip_suffix("/*"))
    {
        s = parent.trim_end_matches('/');
        if s.is_empty() {
            // The whole path was a root glob (`/**`, `/**/`, `/**/*`, `/*`): the parent directory is the filesystem root
            s = "/";
        }
    }

    if is_glob(s) {
        tracing::warn!(
            path = %raw,
            "sandbox read_only/read_write paths are literal directory grants, not globs; \
             name the directory itself (or put globs under `deny`); skipping"
        );
        return None;
    }

    if s != expanded {
        tracing::info!(
            original = %raw,
            normalized = %s,
            "stripped trailing glob from sandbox allow path"
        );
    }
    Some(PathBuf::from(s))
}

/// `~` and `~/rest` name the current user's home. `~other` stays literal.
/// A home-shaped path with no home directory is skipped rather than granted as a relative `~` directory.
fn expand_user_home(raw: &str, home: Option<&Path>) -> Option<String> {
    let Some(rest) = raw.strip_prefix('~') else {
        return Some(raw.to_string());
    };
    if !rest.is_empty() && !rest.starts_with('/') {
        return Some(raw.to_string());
    }
    let Some(home) = home else {
        tracing::warn!(
            path = %raw,
            "sandbox allow path starts with ~ but no home directory is available; skipping"
        );
        return None;
    };
    let relative = rest.trim_start_matches('/');
    if relative.is_empty() {
        Some(home.to_string_lossy().into_owned())
    } else {
        Some(home.join(relative).to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_allow_path_cases() {
        for (raw, expected) in [
            // One trailing glob group strips to the parent directory.
            ("/home/u/.cargo/cache/**", Some("/home/u/.cargo/cache")),
            ("/home/u/.cargo/cache/**/", Some("/home/u/.cargo/cache")),
            ("/home/u/.cargo/cache/**/*", Some("/home/u/.cargo/cache")),
            ("/tmp/scratch/*", Some("/tmp/scratch")),
            // Root globs grant the filesystem root, their parent directory.
            ("/**", Some("/")),
            ("/**/", Some("/")),
            ("/**/*", Some("/")),
            ("/*", Some("/")),
            // Literal directory paths pass through unchanged.
            ("/tmp/scratch", Some("/tmp/scratch")),
            ("/tmp/scratch/", Some("/tmp/scratch")),
            ("/", Some("/")),
            // Still glob-shaped after one strip: skip, never widen further (stacked wildcards must not collapse to a higher parent)
            ("/a/**/**", None),
            ("/a/*/*", None),
            ("/home/**/cache", None),
            ("/tmp/foo*", None),
            // Surrounding whitespace is rejected, never trimmed
            // Trimming would widen `/tmp/* ` into a grant of /tmp and rewrite `/srv/cache ` into a different directory than configured
            ("/tmp/* ", None),
            ("/srv/cache ", None),
            (" /srv/cache", None),
            ("   ", None),
            // Nothing configured, nothing to grant.
            ("", None),
        ] {
            assert_eq!(
                normalize_allow_path(raw),
                expected.map(PathBuf::from),
                "normalize_allow_path({raw:?})"
            );
        }
    }

    #[test]
    fn tilde_allow_path_expands_to_the_home_directory_and_unknown_keys_fail() {
        let home = PathBuf::from("/home/ada");
        assert_eq!(
            normalize_allow_path_with_home("~/notes/**", Some(home.as_path())),
            Some(home.join("notes"))
        );
        assert_eq!(
            normalize_allow_path_with_home("~", Some(home.as_path())),
            Some(home.clone())
        );
        assert_eq!(
            normalize_allow_path_with_home("~other/notes", Some(home.as_path())),
            Some(PathBuf::from("~other/notes"))
        );
        let user_home = xai_dirs::home_dir().expect("home directory");
        assert_eq!(
            normalize_allow_path("~/notes"),
            Some(user_home.join("notes")),
            "the shipped allow-path parser expands ~/ to the home directory"
        );

        let root = std::env::temp_dir().join(format!(
            "grok-sandbox-unknown-key-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join(".grok")).expect("temp dir");
        let project = root.join(".grok").join("sandbox.toml");
        std::fs::write(
            &project,
            "[profiles.custom]\nread_write = [\"~/notes\"]\nextra = true\n",
        )
        .expect("write sandbox.toml");
        let missing = root.join("no-user-sandbox.toml");
        let errors = crate::sandbox_config_load_errors_at(&missing, &project);
        let parsed = errors.first().expect("unknown key is a load error");
        assert_eq!(parsed.path, project);
        assert!(
            parsed.message.to_ascii_lowercase().contains("unknown"),
            "{}",
            parsed.message
        );
        assert!(parsed.message.contains("extra"), "{}", parsed.message);
        let _ = std::fs::remove_dir_all(root);
    }
}
