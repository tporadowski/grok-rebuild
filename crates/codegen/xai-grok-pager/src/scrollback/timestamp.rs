//! Clock text drawn on the first line of a user or agent message.

use chrono::{DateTime, Local, TimeZone};

use crate::appearance::TimestampMode;

/// Short clock, or the full date when `on` and the pointer is on the clock. `off` draws nothing.
pub fn message_timestamp_text(
    mode: TimestampMode,
    hovered: bool,
    ts: DateTime<Local>,
) -> Option<String> {
    match mode {
        TimestampMode::Off => None,
        TimestampMode::Minimal => Some(ts.format("  %-I:%M %p").to_string()),
        TimestampMode::On if hovered => Some(ts.format("  %H:%M:%S | %b %d").to_string()),
        TimestampMode::On => Some(ts.format("  %-I:%M %p").to_string()),
    }
}

/// Columns held back so wrapped text does not run under the clock. The full date is hover-only and wider.
pub fn message_timestamp_reserved(mode: TimestampMode, block_shows_clock: bool) -> u16 {
    if mode.shows() && block_shows_clock { 10 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 8, 15, 4, 5).unwrap()
    }

    #[test]
    fn minimal_keeps_the_short_clock_and_on_expands_it_on_hover() {
        let ts = sample();
        let short = message_timestamp_text(TimestampMode::Minimal, false, ts).unwrap();
        assert_eq!(
            message_timestamp_text(TimestampMode::Minimal, true, ts).as_deref(),
            Some(short.as_str()),
            "minimal does not grow the clock under the pointer"
        );
        assert_eq!(
            message_timestamp_text(TimestampMode::On, false, ts).as_deref(),
            Some(short.as_str()),
            "on uses the same short clock until the pointer is on it"
        );
        let full = message_timestamp_text(TimestampMode::On, true, ts).unwrap();
        assert_ne!(full, short);
        assert!(
            full.contains("15:04:05"),
            "hover on the on mode shows the seconds: {full}"
        );
        assert!(message_timestamp_text(TimestampMode::Off, true, ts).is_none());
        assert_eq!(message_timestamp_reserved(TimestampMode::Off, true), 0);
        assert_eq!(message_timestamp_reserved(TimestampMode::Minimal, true), 10);
        assert_eq!(message_timestamp_reserved(TimestampMode::On, false), 0);
    }
}
