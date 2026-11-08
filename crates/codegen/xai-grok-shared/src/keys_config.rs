//! `[keys]` settings. Vim navigation is one value: `[keys] vim`.
//! `[ui] vim_mode` is the older boolean and still loads when `[keys] vim` is absent.

/// `off`, `nav`, or `all`. `all` is the prompt-editing mode from 1.0.71; navigation treats it like `nav`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimKeys {
    Off,
    Nav,
    All,
}

impl VimKeys {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Nav => "nav",
            Self::All => "all",
        }
    }

    /// Scrollback vim keys and suggestion-key changes. `all` includes navigation.
    pub fn navigates(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "off" | "false" | "0" => Some(Self::Off),
            "nav" | "on" | "true" | "navigation" => Some(Self::Nav),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

/// `[keys] vim` wins when it parses. An absent or unrecognized value falls back to `[ui] vim_mode`, then `off`.
pub fn resolve_vim_keys(keys_vim: Option<&str>, legacy_vim_mode: Option<bool>) -> VimKeys {
    if let Some(parsed) = keys_vim.and_then(VimKeys::parse) {
        return parsed;
    }
    match legacy_vim_mode {
        Some(true) => VimKeys::Nav,
        Some(false) | None => VimKeys::Off,
    }
}

/// What a bool toggle writes. `true` is `nav` (not `all`).
pub fn vim_keys_for_bool(enabled: bool) -> &'static str {
    if enabled { "nav" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_vim_wins_over_the_legacy_bool() {
        assert_eq!(resolve_vim_keys(Some("nav"), Some(false)), VimKeys::Nav);
        assert_eq!(resolve_vim_keys(Some("off"), Some(true)), VimKeys::Off);
        assert!(resolve_vim_keys(Some("NAV"), Some(false)).navigates());
    }

    #[test]
    fn legacy_bool_is_nav_when_keys_vim_is_absent() {
        assert_eq!(resolve_vim_keys(None, Some(true)), VimKeys::Nav);
        assert_eq!(resolve_vim_keys(None, Some(false)), VimKeys::Off);
        assert_eq!(resolve_vim_keys(None, None), VimKeys::Off);
        assert_eq!(
            resolve_vim_keys(Some("nope"), Some(true)),
            VimKeys::Nav,
            "an unrecognized keys value falls back to the bool"
        );
    }

    #[test]
    fn all_navigates_like_nav() {
        let all = resolve_vim_keys(Some("all"), None);
        assert_eq!(all, VimKeys::All);
        assert!(all.navigates());
        assert_eq!(vim_keys_for_bool(true), "nav");
        assert_eq!(vim_keys_for_bool(false), "off");
    }
}
