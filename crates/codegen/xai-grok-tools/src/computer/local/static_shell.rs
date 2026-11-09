//! Static (replay-only) login-shell capture for the non-persistent bash path.
//!
//! Sources the user's rc once at init and captures function and alias
//! definitions; every command replays that fixed snapshot in a fresh shell.
//! Nothing is ever written back: no state dump, no tracked cwd, no
//! persistence across calls. Env vars are deliberately not captured here —
//! the host-side login env capture applies them with fill-gaps precedence.
//!
//! Self-contained by design: independent of the cursor persistent shell's
//! `shell_state` machinery so changes to either path cannot affect the other.

use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
use std::path::Path;
use std::time::Duration;

use base64::Engine as _;
use command_fds::FdMapping;
use nix::libc;
use tokio::io::AsyncReadExt;

pub use xai_grok_config::shell::UnixShellKind;

const INIT_MARKER: &str = "__GROK_STATIC_SHELL_MARKER__";
const INIT_TIMEOUT: Duration = Duration::from_secs(15);

/// A fixed snapshot of rc-defined functions and aliases, captured once.
#[derive(Debug, Clone)]
pub struct StaticShellSnapshot {
    pub snapshot: String,
    pub shell: UnixShellKind,
}

fn shell_binary(shell: UnixShellKind) -> &'static str {
    xai_grok_config::shell::unix_shell_path(shell)
}

fn rc_file_name(shell: UnixShellKind) -> &'static str {
    match shell {
        UnixShellKind::Bash => ".bashrc",
        UnixShellKind::Zsh => ".zshrc",
    }
}

fn sudo_alias_injection() -> String {
    match std::env::var("SUDO_ASKPASS") {
        Ok(val) if !val.is_empty() => "alias sudo='sudo -A'; ".to_string(),
        _ => String::new(),
    }
}

fn capture_command(shell: UnixShellKind) -> &'static str {
    match shell {
        UnixShellKind::Bash => "builtin alias -p 2>/dev/null; builtin declare -f 2>/dev/null",
        UnixShellKind::Zsh => {
            "{ builtin alias -L; builtin alias -gL; builtin alias -sL } 2>/dev/null; \
             builtin typeset -f 2>/dev/null"
        }
    }
}

/// Print the alias and function capture as one base64 blob between markers.
/// `declare -f` emits a raw SOH byte for a body that contains one. Splitting
/// the capture on that byte kept only the text before the first such function.
fn framed_capture(capture: &str) -> String {
    format!(
        "printf '%s\\n' '{marker}'; {{ {capture}; }} 2>/dev/null | command base64 | command tr -d '\\n'; printf '\\n%s\\n' '{marker}'",
        marker = INIT_MARKER,
        capture = capture,
    )
}

fn static_capture_script(shell: UnixShellKind) -> String {
    format!(
        "source \"$HOME/{rc}\" 2>/dev/null; {framed}",
        rc = rc_file_name(shell),
        framed = framed_capture(capture_command(shell)),
    )
}

/// Decode the base64 blob between the capture markers. Missing markers or
/// undecodable text become an empty snapshot.
fn snapshot_from_capture_stdout(stdout: &str) -> String {
    let marker_line = format!("{INIT_MARKER}\n");
    let Some((_, rest)) = stdout.split_once(&marker_line) else {
        return String::new();
    };
    let Some((payload, _)) = rest.split_once(&marker_line) else {
        return String::new();
    };
    let b64: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    if b64.is_empty() {
        return String::new();
    }
    match base64::engine::general_purpose::STANDARD.decode(b64) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => String::new(),
    }
}

impl StaticShellSnapshot {
    /// Source the rc once in a login shell and capture alias and function
    /// definitions as base64 between markers. Returns an empty snapshot on any
    /// failure or timeout, degrading to a plain shell.
    pub async fn init(
        cwd: &Path,
        sandbox_hook: Option<&dyn crate::sandbox_launch::SandboxLaunch>,
    ) -> Self {
        let shell = xai_grok_config::shell::detect_unix_shell_kind();

        let script = static_capture_script(shell);

        let result = tokio::time::timeout(INIT_TIMEOUT, async {
            let mut cmd = tokio::process::Command::new(shell_binary(shell));
            cmd.args(["-lc", &script]).current_dir(cwd);
            cmd.envs(crate::util::pager_env());
            let call = crate::sandbox_launch::CallId::shell_init("static-shell");
            crate::sandbox_launch::prepare(sandbox_hook, &mut cmd, &call)
                .map_err(|reason| {
                    tracing::warn!(%reason, "static shell capture refused by the sandbox hook")
                })
                .ok()?;
            crate::sandbox_launch::wire_prepared(&mut cmd, xai_tty_utils::null_stdio());
            #[allow(clippy::disallowed_methods)] // probe killed on drop
            let mut child = cmd.spawn().ok()?;

            let mut stdout_buf = Vec::new();
            if let Some(ref mut stdout) = child.stdout {
                stdout.read_to_end(&mut stdout_buf).await.ok();
            }
            let status = child.wait().await.ok()?;
            if !status.success() {
                return None;
            }

            let stdout = String::from_utf8_lossy(&stdout_buf);
            if stdout.contains(INIT_MARKER) {
                Some(snapshot_from_capture_stdout(&stdout))
            } else {
                None
            }
        })
        .await;

        let snapshot = match result {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!("static shell capture failed; using empty snapshot");
                String::new()
            }
            Err(_) => {
                tracing::warn!(
                    "static shell capture timed out after {}s; using empty snapshot",
                    INIT_TIMEOUT.as_secs()
                );
                String::new()
            }
        };
        Self { snapshot, shell }
    }

    /// Build the replay wrapper: read the snapshot from fd 3, eval it (alias and function
    /// definitions), then eval the user command; the shell exits with the user command's status. A
    /// failing snapshot replay does not abort the command.
    pub fn prepare_command(
        &self,
        user_command: &str,
        search_shadows: super::SearchShadowConfig,
    ) -> std::io::Result<PreparedStaticCommand> {
        let sudo_inject = sudo_alias_injection();
        let search_inject = super::embedded_search_tools::search_injection(search_shadows);

        let (state_in_read, state_in_write) = os_pipe()?;
        set_cloexec(&state_in_write)?;

        // Copy $1 into a plain variable and clear the positional parameters (`builtin set --`) BEFORE eval'ing the user command: `source <script>`
        // with no arguments makes the sourced script inherit the caller's positional parameters, so e.g. conda's `bin/activate` (which forwards "$@"
        // to `conda activate`) would receive the entire wrapped command string as an environment name.
        let wrapper = match self.shell {
            UnixShellKind::Bash => format!(
                "snap=$(command cat <&3); builtin shopt -s extglob 2>/dev/null; \
                 builtin shopt -s expand_aliases 2>/dev/null; \
                 builtin eval -- \"$snap\"; \
                 builtin export GROK_AGENT=1; \
                 builtin export PWD=\"$(builtin pwd)\"; {sudo_inject}{search_inject}\
                 __grok_user_cmd=\"$1\"; builtin declare +x __grok_user_cmd 2>/dev/null; builtin set --; \
                 builtin eval \"$__grok_user_cmd\" 2>&1"
            ),
            UnixShellKind::Zsh => format!(
                // A fresh zsh defines `alias run-help=man`. Startup files that replace it
                // (`unalias run-help; autoload run-help`) replay a function of that name.
                // Defining the function while the alias still exists is a parse error, and
                // every alias or function after it in the snapshot is dropped. Clear aliases
                // before the replay; the snapshot puts them back, including `alias grep`.
                "snap=$(command cat <&3); \
                 builtin unsetopt aliases 2>/dev/null; \
                 builtin unalias -m '*' 2>/dev/null || true; \
                 builtin setopt nonomatch 2>/dev/null; \
                 builtin eval \"$snap\"; \
                 builtin export GROK_AGENT=1; \
                 builtin export PWD=\"$(builtin pwd)\"; \
                 builtin setopt aliases 2>/dev/null; {sudo_inject}{search_inject}\
                 __grok_user_cmd=\"$1\"; builtin typeset +x __grok_user_cmd 2>/dev/null; builtin set --; \
                 builtin eval \"$__grok_user_cmd\" 2>&1"
            ),
        };

        let args: Vec<String> = match self.shell {
            UnixShellKind::Bash => vec![
                "-O".into(),
                "extglob".into(),
                "-c".into(),
                wrapper,
                "--".into(),
                user_command.into(),
            ],
            UnixShellKind::Zsh => vec!["-c".into(), wrapper, "--".into(), user_command.into()],
        };

        Ok(PreparedStaticCommand {
            binary: shell_binary(self.shell).to_string(),
            args,
            fd_mappings: vec![FdMapping {
                parent_fd: state_in_read,
                child_fd: 3,
            }],
            state_in_write,
        })
    }
}

pub struct PreparedStaticCommand {
    pub binary: String,
    pub args: Vec<String>,
    pub fd_mappings: Vec<FdMapping>,
    pub state_in_write: OwnedFd,
}

/// Write the snapshot to the pipe, then close the fd so the child sees EOF.
pub async fn write_snapshot_to_pipe(snapshot: &str, fd: OwnedFd) -> std::io::Result<()> {
    let data = snapshot.to_string();
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        // Safety: we own the fd.
        let mut file = unsafe { std::fs::File::from_raw_fd(fd.as_raw_fd()) };
        std::mem::forget(fd);
        file.write_all(data.as_bytes())?;
        file.flush()?;
        drop(file);
        Ok(())
    })
    .await
    .map_err(std::io::Error::other)?
}

fn os_pipe() -> std::io::Result<(OwnedFd, OwnedFd)> {
    #[cfg(target_os = "linux")]
    {
        nix::unistd::pipe2(nix::fcntl::OFlag::O_CLOEXEC)
            .map_err(|e| std::io::Error::from_raw_os_error(e as i32))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let (read_fd, write_fd) =
            nix::unistd::pipe().map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
        let _ = set_cloexec(&read_fd);
        let _ = set_cloexec(&write_fd);
        Ok((read_fd, write_fd))
    }
}

fn set_cloexec(fd: &OwnedFd) -> std::io::Result<()> {
    let raw = fd.as_raw_fd();
    let flags = unsafe { libc::fcntl(raw, libc::F_GETFD) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let ret = unsafe { libc::fcntl(raw, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
#[path = "static_shell_cli_facing_unchanged_tests.rs"]
mod cli_facing_unchanged_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use command_fds::CommandFdExt;
    use std::process::Stdio;

    fn bash_available() -> bool {
        std::path::Path::new("/bin/bash").exists()
    }

    async fn run_static_with(
        shell: UnixShellKind,
        snapshot: &str,
        command: &str,
    ) -> std::process::Output {
        let state = StaticShellSnapshot {
            snapshot: snapshot.to_string(),
            shell,
        };
        let prep = state
            .prepare_command(
                command,
                crate::computer::local::SearchShadowConfig::default(),
            )
            .unwrap();
        let mut cmd = tokio::process::Command::new(&prep.binary);
        cmd.args(&prep.args)
            .current_dir(std::env::current_dir().unwrap())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        cmd.fd_mappings(prep.fd_mappings).unwrap();
        #[allow(clippy::disallowed_methods)] // test fixture; the test reaps it
        let child = cmd.spawn().unwrap();
        drop(cmd);

        let snap = state.snapshot.clone();
        let write_handle =
            tokio::spawn(async move { write_snapshot_to_pipe(&snap, prep.state_in_write).await });
        let output = child.wait_with_output().await.unwrap();
        write_handle.await.unwrap().unwrap();
        output
    }

    async fn run_static(snapshot: &str, command: &str) -> std::process::Output {
        run_static_with(UnixShellKind::Bash, snapshot, command).await
    }

    fn zsh_available() -> bool {
        std::path::Path::new(xai_grok_config::shell::unix_shell_path(UnixShellKind::Zsh)).exists()
    }

    /// Startup files that alias `grep` and replace zsh's `run-help` alias must not
    /// drop the rest of the snapshot. A fresh zsh still has `alias run-help=man`.
    #[tokio::test]
    async fn zsh_grep_alias_and_replaced_run_help_still_runs() {
        if !zsh_available() {
            return;
        }
        let snapshot = "\
alias grep='grep --color=auto'\n\
run-help () {\n\
\t# undefined\n\
\tbuiltin autoload -XUz\n\
}\n\
later_fn () { echo LATER_OK; }\n\
";
        let output = run_static_with(
            UnixShellKind::Zsh,
            snapshot,
            "later_fn; printf 'hello\\n' | grep hello",
        )
        .await;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "command failed: stdout={stdout:?} stderr={stderr:?}"
        );
        assert!(
            !stderr.contains("defining function based on alias"),
            "run-help must not be a parse error: {stderr:?}"
        );
        assert!(
            stdout.contains("LATER_OK"),
            "function after run-help must still load: {stdout:?}"
        );
        assert!(
            stdout.contains("hello"),
            "aliased grep must still print the match: {stdout:?}"
        );
        assert!(
            !stdout.contains("defining function based on alias"),
            "run-help must not be a parse error: {stdout:?}"
        );
    }

    /// A startup function whose body contains a raw SOH byte, an extglob
    /// pattern, or `!` must still be defined, and so must a function after it.
    #[tokio::test]
    async fn startup_functions_with_special_characters_survive_static_capture() {
        if !bash_available() {
            return;
        }
        let framed = framed_capture(capture_command(UnixShellKind::Bash));
        let script = format!(
            "builtin shopt -s extglob\n\
             f_at() {{ echo @(a|b); }}\n\
             f_bang() {{ printf '%s\\n' 'hello!'; }}\n\
             f_raw() {{ printf $'\\001'; }}\n\
             later() {{ printf '%s\\n' LATER_OK; }}\n\
             {framed}\n"
        );
        let output = tokio::process::Command::new("bash")
            .args(["--noprofile", "--norc", "-c", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "capture failed: stdout={stdout:?} stderr={stderr:?}"
        );
        let snapshot = snapshot_from_capture_stdout(&stdout);
        assert!(
            snapshot.contains("LATER_OK"),
            "function after a special byte must stay in the snapshot: {snapshot:?}"
        );
        assert!(
            snapshot.as_bytes().contains(&1),
            "the SOH byte in the function body must round-trip: {snapshot:?}"
        );
        let output = run_static(&snapshot, "later; f_bang; f_at; f_raw | od -An -tx1").await;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "replay failed: stdout={stdout:?} stderr={stderr:?}"
        );
        assert!(stdout.contains("LATER_OK"), "{stdout:?}");
        assert!(stdout.contains("hello!"), "{stdout:?}");
        assert!(stdout.contains("@(a|b)"), "{stdout:?}");
        assert!(
            stdout.contains("01"),
            "replayed function must still emit SOH: {stdout:?}"
        );
    }

    #[tokio::test]
    async fn replays_aliases_and_functions() {
        if !bash_available() {
            return;
        }
        let output = run_static(
            "alias grok_alias_probe='echo ALIAS_OK'\ngrok_fn_probe() { echo FN_OK; }\n",
            "grok_alias_probe && grok_fn_probe",
        )
        .await;
        assert!(output.status.success(), "command failed: {output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("ALIAS_OK") && stdout.contains("FN_OK"),
            "alias and function must be replayed: {stdout:?}"
        );
    }

    #[tokio::test]
    async fn user_command_exit_code_propagates_past_bad_snapshot() {
        if !bash_available() {
            return;
        }
        let output = run_static("this-is-not-a-command 2>/dev/null\n", "exit 7").await;
        assert_eq!(
            output.status.code(),
            Some(7),
            "user command exit code must propagate: {output:?}"
        );
    }

    #[tokio::test]
    async fn empty_snapshot_runs_plain() {
        if !bash_available() {
            return;
        }
        let output = run_static("", "echo PLAIN_OK").await;
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("PLAIN_OK"),
            "empty snapshot must degrade to a plain shell"
        );
    }

    /// Regression test: a script sourced WITHOUT arguments by the user command must not see the wrapper's positional parameters ($1 = the whole
    /// command string). Conda's `bin/activate` forwards "$@" to `conda activate`, so a leak makes every `activate_conda`-prefixed command fail with
    /// `EnvironmentLocationNotFound: Not a conda environment: <cwd>/<the entire command string>`.
    #[tokio::test]
    async fn sourced_script_does_not_inherit_wrapper_positional_args() {
        if !bash_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let probe = dir.path().join("activate_probe.sh");
        std::fs::write(&probe, "echo \"SOURCED_ARGC=$#\"\n").unwrap();

        let output = run_static(
            "",
            &format!("source {} && echo AFTER_SOURCE_OK", probe.display()),
        )
        .await;
        assert!(output.status.success(), "command failed: {output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SOURCED_ARGC=0"),
            "sourced script must see zero positional args, got: {stdout:?}"
        );
        assert!(stdout.contains("AFTER_SOURCE_OK"), "got: {stdout:?}");
    }
}
