use std::path::PathBuf;
use std::sync::OnceLock;
use xai_tool_runtime::{ToolError, ToolErrorKind};

#[cfg(bundle_rg)]
const RG_BYTES: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/bundle-rg/rg-",
    env!("GROK_TOOLS_RG_VER"),
    "-",
    env!("GROK_TOOLS_RG_TARGET"),
    ".bin.zst"
));

#[cfg(bundle_rg)]
fn resolve_bundled_rg() -> Result<Option<PathBuf>, crate::util::vendor::InstallError> {
    crate::util::vendor::resolve(
        concat!(
            "rg-",
            env!("GROK_TOOLS_RG_VER"),
            "-",
            env!("GROK_TOOLS_RG_TARGET")
        ),
        RG_BYTES,
        env!("GROK_TOOLS_RG_SHA256"),
    )
}

pub fn rg_path() -> Result<PathBuf, ToolError> {
    static RG_EXEC: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    RG_EXEC
        .get_or_init(|| {
            #[cfg(bundle_rg)]
            {
                resolve_bundled_rg()
                    .map(|found| found.unwrap_or_else(|| PathBuf::from("rg")))
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(bundle_rg))]
            {
                Ok(rg_from_path_or_runfiles())
            }
        })
        .clone()
        .map_err(|msg| ToolError::new(ToolErrorKind::Execution, msg))
}

#[cfg(not(bundle_rg))]
fn rg_from_path_or_runfiles() -> PathBuf {
    if let Ok(p) = std::env::var("RG_BIN_PATH") {
        return PathBuf::from(p);
    }
    if let Ok(rf) = std::env::var("RUNFILES_DIR")
        && let Ok(entries) = std::fs::read_dir(PathBuf::from(rf))
    {
        for entry in entries.flatten() {
            if entry
                .file_name()
                .to_string_lossy()
                .contains("ripgrep_hermetic")
            {
                for sub in ["amd64/rg", "arm64/rg", "rg"] {
                    let candidate = entry.path().join(sub);
                    if candidate.exists() {
                        return candidate;
                    }
                }
            }
        }
    }
    PathBuf::from("rg")
}

/// Directory of the extracted bundled `rg`, when this build has one on disk.
/// A bare `rg` name (the PATH fallback) is not a directory to inject.
pub fn bundled_rg_dir() -> Option<PathBuf> {
    let path = rg_path().ok()?;
    if !path.is_absolute() || !path.is_file() {
        return None;
    }
    path.parent().map(std::path::Path::to_path_buf)
}

/// Append `dir` to a `PATH` string when that directory is not already an entry.
pub fn append_os_path(path: &std::ffi::OsStr, dir: &std::path::Path) -> std::ffi::OsString {
    if std::env::split_paths(path).any(|entry| entry == dir) {
        return path.to_os_string();
    }
    let mut out = path.to_os_string();
    if !out.is_empty() {
        out.push(if cfg!(windows) { ";" } else { ":" });
    }
    out.push(dir);
    out
}

#[cfg(test)]
mod tests {
    use super::append_os_path;
    use std::ffi::{OsStr, OsString};
    use std::path::Path;

    #[test]
    fn append_os_path_adds_a_missing_dir_once() {
        let dir = Path::new("/opt/grok-rg");
        let sep = if cfg!(windows) { ";" } else { ":" };
        assert_eq!(
            append_os_path(OsStr::new(""), dir),
            OsString::from("/opt/grok-rg")
        );
        let once = append_os_path(OsStr::new("/usr/bin"), dir);
        assert_eq!(once, OsString::from(format!("/usr/bin{sep}/opt/grok-rg")));
        assert_eq!(append_os_path(&once, dir), once);
    }
}
