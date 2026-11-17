//! Startup cursor-position question (`CSI 6 n`).
//!
//! Crossterm waits two seconds for the report. Inline and minimal startup ask with a short budget
//! and, if the terminal is still silent, anchor the viewport on the bottom row so the first frame paints.

use std::time::Duration;

/// Long enough for a local terminal, short enough that a slow reply does not hold the first frame.
/// Crossterm's own poll is two seconds (`cursor/sys/unix.rs`).
pub const STARTUP_CURSOR_QUERY_BUDGET: Duration = Duration::from_millis(50);

/// Ask once, within [`STARTUP_CURSOR_QUERY_BUDGET`]. `None` when the terminal is not a tty, the
/// write fails, or the report does not arrive inside the budget. Coordinates are 0-based `(x, y)`.
pub fn query_startup_cursor() -> Option<(u16, u16)> {
    #[cfg(unix)]
    {
        if !super::probe::write_query(b"\x1b[6n") {
            return None;
        }
        let reply = super::probe::read_tty_reply(STARTUP_CURSOR_QUERY_BUDGET, |buf, _| {
            parse_cursor_report(buf).is_some()
        });
        reply.as_deref().and_then(parse_cursor_report)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// `CSI <row> ; <col> R`, 1-based, anywhere in `buf`. Returns 0-based `(x, y)`.
pub fn parse_cursor_report(buf: &[u8]) -> Option<(u16, u16)> {
    let mut i = 0;
    while i + 4 < buf.len() {
        if buf.get(i) == Some(&0x1b) && buf.get(i + 1) == Some(&b'[') {
            if let Some(pos) = parse_cpr_at(buf.get(i + 2..)?) {
                return Some(pos);
            }
        }
        i += 1;
    }
    None
}

fn parse_cpr_at(rest: &[u8]) -> Option<(u16, u16)> {
    let mut idx = 0;
    let row = read_u16(rest, &mut idx)?;
    if rest.get(idx) != Some(&b';') {
        return None;
    }
    idx += 1;
    let col = read_u16(rest, &mut idx)?;
    if rest.get(idx) != Some(&b'R') {
        return None;
    }
    Some((col.saturating_sub(1), row.saturating_sub(1)))
}

fn read_u16(buf: &[u8], idx: &mut usize) -> Option<u16> {
    let start = *idx;
    while buf.get(*idx).is_some_and(|byte| byte.is_ascii_digit()) {
        *idx += 1;
    }
    if *idx == start {
        return None;
    }
    let text = std::str::from_utf8(buf.get(start..*idx)?).ok()?;
    text.parse().ok()
}

/// Cursor used to place the inline viewport at startup.
/// A report that arrived inside the budget is kept. Silence uses the bottom row, where the shell prompt sits.
pub fn startup_cursor_anchor(reported: Option<(u16, u16)>, rows: u16) -> (u16, u16) {
    reported.unwrap_or((0, rows.saturating_sub(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_cursor_budget_does_not_wait_out_a_slow_reply() {
        assert!(STARTUP_CURSOR_QUERY_BUDGET <= Duration::from_millis(50));
        assert!(STARTUP_CURSOR_QUERY_BUDGET < Duration::from_secs(2));
        // Silence stands in for a terminal that has not answered yet. The first frame anchors
        // at the bottom instead of blocking on crossterm's two-second poll.
        assert_eq!(startup_cursor_anchor(None, 40), (0, 39));
        assert_eq!(parse_cursor_report(b"\x1b[12;4R"), Some((3, 11)));
        assert_eq!(
            startup_cursor_anchor(parse_cursor_report(b"noise\x1b[2;1R"), 40),
            (0, 1)
        );
        assert_eq!(parse_cursor_report(b"\x1b[6n"), None);
        assert_eq!(parse_cursor_report(b""), None);
        assert_eq!(startup_cursor_anchor(None, 0), (0, 0));
    }
}
