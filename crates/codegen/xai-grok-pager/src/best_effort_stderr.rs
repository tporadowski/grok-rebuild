//! Stderr lines for paths where fd 2 may already be dead.
//!
//! Once stderr's reader is gone (a closed terminal pane, a pipe whose reader exited) every
//! write fails, `eprintln!` panics on that failure, and under `panic = "abort"` the panic is a
//! SIGABRT plus a crash report on the next launch. The exit error report, the slow session-end
//! notice, and headless plain-format diagnostics all run when fd 2 is likely to be exactly that
//! dead terminal, so they write through here: one attempt, and failure is reported, not raised.

use std::io::Write;

/// Write `line` and a newline to `w`; `false` when the write failed.
///
/// The writer is a parameter so callers can be tested against a closed pipe in-process.
pub fn write_line(w: &mut impl Write, line: &str) -> bool {
    writeln!(w, "{line}").is_ok()
}

/// [`write_line`] to process stderr, outcome discarded.
pub fn eprint_line(line: &str) {
    write_line(&mut std::io::stderr(), line);
}

/// True when a panic is the standard-library print macro failing to write.
///
/// `println!` and `eprintln!` panic with `failed printing to stdout` or
/// `failed printing to stderr` for a closed pipe and for any other write error.
/// `grok update` and `grok login` exit on that panic instead of aborting.
pub fn is_stdio_print_failure(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("failed printing to stdout") || lower.contains("failed printing to stderr")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_line_appends_a_newline_and_reports_success() {
        let mut buf = Vec::new();
        assert!(write_line(&mut buf, "Finishing session…"));
        assert_eq!(buf, "Finishing session…\n".as_bytes());
    }

    /// Closing the read end makes every write fail with EPIPE, like the dead tty a closed pane
    /// leaves behind. SIGPIPE is ignored in Rust binaries, so the write returns an error.
    #[test]
    fn closed_or_unwritable_print_panics_are_stdio_failures() {
        assert!(is_stdio_print_failure(
            "failed printing to stdout: Broken pipe (os error 32)"
        ));
        assert!(is_stdio_print_failure(
            "failed printing to stderr: Broken pipe (os error 32)"
        ));
        assert!(is_stdio_print_failure(
            "failed printing to stdout: Permission denied (os error 13)"
        ));
        assert!(is_stdio_print_failure(
            "failed printing to stderr: No space left on device (os error 28)"
        ));
        assert!(!is_stdio_print_failure("config.toml is not a table"));
        assert!(!is_stdio_print_failure(
            "os error 32 while reading the config file"
        ));
    }

    /// Closing the read end makes every write fail with EPIPE, like the dead tty a closed pane
    /// leaves behind. SIGPIPE is ignored in Rust binaries, so the write returns an error.
    #[test]
    fn write_line_reports_failure_on_a_dead_pipe() {
        let (reader, mut writer) = std::io::pipe().expect("pipe");
        drop(reader);
        assert!(!write_line(&mut writer, "unreachable"));
    }
}
