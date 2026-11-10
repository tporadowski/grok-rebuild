//! Structured file-log layers and the shared non-blocking file appender.

pub(crate) mod appender;
pub mod debug_log;
pub mod hooks_log;
pub mod memory_log;
pub mod sampling_log;
pub mod unified_log;

/// `NO_COLOR` is set to a non-empty value. An empty value does not disable color.
#[must_use]
pub fn no_color_is_set() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
}

/// A log line is colored only when its writer is a terminal and `NO_COLOR` is unset.
/// A file, a redirected stderr, and `NO_COLOR` stay plain text.
#[must_use]
pub fn log_line_uses_ansi(writer_is_terminal: bool, no_color: bool) -> bool {
    writer_is_terminal && !no_color
}

#[cfg(test)]
mod tests {
    use super::log_line_uses_ansi;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("log buffer").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedBuf {
        type Writer = SharedBuf;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture_log_line(ansi: bool) -> String {
        use tracing_subscriber::layer::SubscriberExt as _;
        let buf = SharedBuf(Arc::new(Mutex::new(Vec::<u8>::new())));
        let layer = tracing_subscriber::fmt::layer()
            .with_ansi(ansi)
            .with_target(false)
            .without_time()
            .with_writer(buf.clone());
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("plain log line");
        });
        let bytes = buf.0.lock().expect("log buffer").clone();
        String::from_utf8(bytes).expect("log line is utf-8")
    }

    #[test]
    fn log_lines_stay_plain_for_a_file_and_under_no_color() {
        let file = capture_log_line(log_line_uses_ansi(false, false));
        let no_color = capture_log_line(log_line_uses_ansi(true, true));
        let terminal = capture_log_line(log_line_uses_ansi(true, false));
        assert!(file.contains("plain log line"), "{file}");
        assert!(no_color.contains("plain log line"), "{no_color}");
        assert!(!file.contains('\u{1b}'), "{file}");
        assert!(!no_color.contains('\u{1b}'), "{no_color}");
        assert!(
            terminal.contains('\u{1b}'),
            "a terminal without NO_COLOR keeps color codes: {terminal}"
        );
        assert!(!log_line_uses_ansi(false, true));
    }
}
