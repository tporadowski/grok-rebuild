use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use xai_grok_config::mcp_servers::McpServerOrigin;

/// Where a piece of configuration was loaded from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ConfigSource {
    /// Built-in / bundled with the binary.
    Builtin,
    /// Bundled skill shipped with the binary (extracted to ~/.grok/skills/
    /// or injected via bundled skill dirs).
    Bundled { path: PathBuf },
    /// Server-synced (e.g. ~/.grok/server-skills from the skill store).
    Server { path: PathBuf },
    /// Project-scoped: cwd/.grok/ or cwd/.claude/.
    Project { path: PathBuf },
    /// User-scoped: ~/.grok/ or ~/.claude/.
    User { path: PathBuf },
    /// Plugin-provided component.
    Plugin { plugin_name: String, path: PathBuf },
    /// config.toml `[mcp_servers.*]`, `[skills]`, etc. `path` is
    /// domain-specific: the declaring config.toml for MCP servers, the
    /// skill's own SKILL.md for `[skills].paths` skills.
    ConfigToml { path: PathBuf },
    /// `~/.claude.json` MCP servers.
    ClaudeJson { path: PathBuf },
    /// `.mcp.json` project-level MCP config.
    McpJson { path: PathBuf },
    /// CLI override (`--plugin-dir`, `--mcp-server`).
    Cli { path: PathBuf },
    /// Managed (server-managed / IT-deployed).
    Managed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,
    },
}

impl ConfigSource {
    /// Short human-readable label for terminal display.
    pub fn display_short(&self) -> String {
        match self {
            Self::Builtin => "builtin".into(),
            Self::Bundled { path } => format!("bundled: {}", path.display()),
            Self::Server { path } => format!("server: {}", path.display()),
            Self::Project { path } => format!("project: {}", path.display()),
            Self::User { path } => format!("user: {}", path.display()),
            Self::Plugin { plugin_name, .. } => format!("plugin: {plugin_name}"),
            Self::ConfigToml { path } => format!("config: {}", path.display()),
            Self::ClaudeJson { path } => format!("claude: {}", path.display()),
            Self::McpJson { path } => format!(".mcp.json: {}", path.display()),
            Self::Cli { .. } => "cli".into(),
            Self::Managed { .. } => "managed".into(),
        }
    }

    /// Compact label for columnar terminal display (no paths).
    pub fn display_label(&self) -> String {
        match self {
            Self::Builtin => "builtin".into(),
            Self::Bundled { .. } => "bundled".into(),
            Self::Server { .. } => "server".into(),
            Self::Project { .. } => "project".into(),
            Self::User { .. } => "user".into(),
            Self::Plugin { plugin_name, .. } => format!("plugin: {plugin_name}"),
            Self::ConfigToml { .. } => "config".into(),
            Self::ClaudeJson { .. } => "~/.claude.json".into(),
            Self::McpJson { .. } => ".mcp.json".into(),
            Self::Cli { .. } => "cli".into(),
            Self::Managed { .. } => "managed".into(),
        }
    }

    /// Plugin name if this is a plugin-provided component, `None` otherwise.
    pub fn plugin_name(&self) -> Option<&str> {
        match self {
            Self::Plugin { plugin_name, .. } => Some(plugin_name),
            _ => None,
        }
    }

    /// Filesystem path, if any.
    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            Self::Builtin => None,
            Self::Bundled { path }
            | Self::Server { path }
            | Self::Project { path }
            | Self::User { path }
            | Self::Plugin { path, .. }
            | Self::ConfigToml { path }
            | Self::ClaudeJson { path }
            | Self::McpJson { path }
            | Self::Cli { path } => Some(path),
            Self::Managed { path } => path.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_label_names_the_declaring_file() {
        let project = PathBuf::from("/repo/.grok/config.toml");
        let json = PathBuf::from("/repo/.mcp.json");
        let claude = PathBuf::from("/repo/.claude.json");
        assert_eq!(
            ConfigSource::ConfigToml {
                path: project.clone()
            }
            .display_short(),
            "config: /repo/.grok/config.toml"
        );
        assert_eq!(
            ConfigSource::McpJson { path: json }.display_short(),
            ".mcp.json: /repo/.mcp.json"
        );
        assert_eq!(
            ConfigSource::ClaudeJson { path: claude }.display_short(),
            "claude: /repo/.claude.json"
        );
        assert_eq!(
            ConfigSource::from(McpServerOrigin::ConfigToml { path: project }).display_short(),
            "config: /repo/.grok/config.toml"
        );
    }
}

impl From<McpServerOrigin> for ConfigSource {
    fn from(origin: McpServerOrigin) -> ConfigSource {
        match origin {
            McpServerOrigin::ConfigToml { path } => ConfigSource::ConfigToml { path },
            McpServerOrigin::Plugin { plugin_name, path } => {
                ConfigSource::Plugin { plugin_name, path }
            }
            McpServerOrigin::ClaudeJson { path } => ConfigSource::ClaudeJson { path },
            McpServerOrigin::McpJson { path } => ConfigSource::McpJson { path },
        }
    }
}
