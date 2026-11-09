//! Load `config.toml` as a [`toml_edit::DocumentMut`] for in-place edits.
//! A non-blank file that does not parse is left untouched (`None`).

use std::path::Path;

#[must_use]
pub(crate) fn read_config_document_for_edit(path: &Path) -> Option<toml_edit::DocumentMut> {
    #[allow(clippy::manual_unwrap_or_default)]
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => String::new(),
    };
    match content.parse() {
        Ok(d) => Some(d),
        Err(e) => {
            if content.trim().is_empty() {
                return Some(toml_edit::DocumentMut::new());
            }
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "config.toml is not valid TOML; refusing to overwrite"
            );
            None
        }
    }
}

/// Set `[hints].<key>` to `value` in `~/.grok/config.toml`, preserving every other key and table. No-ops when the
/// existing file is non-blank but unparseable, so a malformed config is never clobbered.
pub(crate) fn set_hint(key: &str, value: impl Into<toml_edit::Value>) -> std::io::Result<()> {
    let path =
        xai_grok_tools::util::grok_home::grok_home().join(xai_grok_config::USER_CONFIG_FILENAME);
    set_hint_at(&path, key, value)
}

/// Core of [`set_hint`]; takes the path so tests can point it at a temp dir.
/// Re-reads under the config-init flock so a concurrent settings save keeps MCP
/// servers and permission rules.
fn set_hint_at(path: &Path, key: &str, value: impl Into<toml_edit::Value>) -> std::io::Result<()> {
    let value = value.into();
    xai_grok_shell::util::config::edit_config_toml_locked(path, true, |doc| {
        if let Some(table) = doc
            .entry("hints")
            .or_insert(toml_edit::table())
            .as_table_mut()
        {
            table.insert(key, toml_edit::value(value));
        }
        true
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn merge_round_trip_preserves_sibling_tables() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        fs::write(
            &path,
            "[ui]\ncompact_mode = false\n\n[mcpServers]\nx = \"y\"\n",
        )
        .unwrap();

        let mut doc = read_config_document_for_edit(&path).expect("parse");
        if let Some(ui) = doc.get_mut("ui").and_then(|i| i.as_table_mut()) {
            ui.insert("show_timestamps", toml_edit::value(false));
        }
        fs::write(&path, doc.to_string()).unwrap();

        let body = fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("show_timestamps") && body.contains("mcpServers"),
            "expected merged TOML, got:\n{body}"
        );
    }

    #[test]
    fn nonempty_unparseable_returns_none_and_leaves_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        let bad = "this is [not valid toml\n";
        fs::write(&path, bad).unwrap();

        assert!(read_config_document_for_edit(&path).is_none());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);
    }

    #[test]
    fn missing_file_is_editable_empty_doc() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("absent.toml");
        let doc = read_config_document_for_edit(&path).expect("editable");
        assert!(!doc.contains_key("ui"));
    }

    #[test]
    fn blank_file_is_editable_empty_doc() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        for blank in ["", "   \n", "\n\t  \n"] {
            fs::write(&path, blank).unwrap();
            let doc = read_config_document_for_edit(&path)
                .unwrap_or_else(|| panic!("blank {blank:?} must be an empty document"));
            assert!(
                !doc.contains_key("ui"),
                "blank {blank:?} must not be treated as unparseable"
            );
        }
    }

    #[test]
    fn set_hint_at_round_trips_and_preserves_siblings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        fs::write(&path, "[ui]\ncompact_mode = false\n").unwrap();

        set_hint_at(&path, "memory_modal_fullscreen", true).unwrap();

        let doc = read_config_document_for_edit(&path).expect("reparse");
        assert_eq!(
            doc.get("hints")
                .and_then(|h| h.get("memory_modal_fullscreen"))
                .and_then(|v| v.as_bool()),
            Some(true),
        );
        assert!(
            fs::read_to_string(&path).unwrap().contains("compact_mode"),
            "sibling [ui] should be preserved"
        );
    }

    #[test]
    fn set_hint_at_creates_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/config.toml");
        set_hint_at(&path, "memory_modal_fullscreen", true).unwrap();
        assert!(
            path.exists(),
            "missing file and parent dir should be created"
        );
    }

    #[test]
    fn set_hint_write_then_read_back_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        fs::write(&path, "[ui]\ntheme = \"dark\"\n").unwrap();

        set_hint_at(&path, "memory_modal_fullscreen", true).unwrap();

        let doc = read_config_document_for_edit(&path).expect("reparse");
        let disabled = doc
            .get("hints")
            .and_then(|h| h.get("memory_modal_fullscreen"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(disabled, "should read back true after set_hint write");
    }

    #[test]
    fn set_hint_at_keeps_mcp_and_permission_written_while_it_waits() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        fs::write(&path, "[ui]\ncompact_mode = false\n").unwrap();
        let lock = xai_grok_shell::util::config::acquire_init_lock(dir.path()).unwrap();
        let path_writer = path.clone();
        let writer = std::thread::spawn(move || {
            set_hint_at(&path_writer, "memory_modal_fullscreen", true).unwrap();
        });
        std::thread::sleep(std::time::Duration::from_millis(300));
        let mid = fs::read_to_string(&path).unwrap();
        assert!(
            !mid.contains("memory_modal_fullscreen"),
            "hint save must wait for the config lock, got {mid}"
        );
        fs::write(
            &path,
            "[mcp_servers.linear]\ncommand = \"linear\"\n\n[permission]\nallow = [\"Bash(git status)\"]\n",
        )
        .unwrap();
        drop(lock);
        writer.join().unwrap();
        let body = fs::read_to_string(&path).unwrap();
        let root: toml::Value = toml::from_str(&body).unwrap();
        assert!(
            root.get("mcp_servers")
                .and_then(|servers| servers.get("linear"))
                .is_some(),
            "{body}"
        );
        assert!(
            root.get("permission")
                .and_then(|section| section.get("allow"))
                .and_then(|allow| allow.as_array())
                .is_some_and(|allow| {
                    allow.iter().any(|value| value.as_str() == Some("Bash(git status)"))
                }),
            "{body}"
        );
        assert_eq!(
            root.get("hints")
                .and_then(|hints| hints.get("memory_modal_fullscreen"))
                .and_then(|value| value.as_bool()),
            Some(true),
            "{body}"
        );
    }

    #[test]
    fn set_hint_at_leaves_unparseable_file_untouched() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        let bad = "this is [not valid toml\n";
        fs::write(&path, bad).unwrap();

        // No-op (no write, no clobber) when the existing file cannot be parsed.
        set_hint_at(&path, "memory_modal_fullscreen", true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);
    }

    #[test]
    fn vim_mode_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(xai_grok_config::USER_CONFIG_FILENAME);
        fs::write(&path, "[ui]\ncompact_mode = false\n").unwrap();

        let mut doc = read_config_document_for_edit(&path).expect("parse");
        if let Some(ui) = doc.get_mut("ui").and_then(|i| i.as_table_mut()) {
            ui.insert("vim_mode", toml_edit::value(true));
        }
        fs::write(&path, doc.to_string()).unwrap();

        let doc2 = read_config_document_for_edit(&path).expect("reparse");
        let enabled = doc2
            .get("ui")
            .and_then(|h| h.get("vim_mode"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(enabled, "expected vim_mode = true after round-trip");

        let body = fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("compact_mode"),
            "sibling [ui] keys should be preserved"
        );
    }
}
