//! Which Kitty keyboard enhancement flags the pager negotiates at startup, and the process-global record of the set it actually pushed.
//!
//! "Flags pushed" is not "releases arrive", and conflating them is the bug this module exists to prevent.
//! Alacritty 0.14.x and older is pushed `DISAMBIGUATE_ESCAPE_CODES` without `REPORT_EVENT_TYPES`, so the protocol is live and teardown owes a pop.
//! `Shift+Enter` works, yet no release ever comes, and a hold-to-talk started there could only end on Esc.
//!
//! The gate reads the reported version rather than watching behaviour because there is nothing to watch.
//! A conforming terminal reports no release for these keys either (kitty spec).
//! So healthy and broken differ by one byte per keystroke, gone by the time events are decoded.
//! An earlier design died on it.
//!
//! Deliberately uncovered: [`super::da2`] is skipped under CSI-intercepting multiplexers but [`super::TerminalContext::kitty_skip_reason`] is not.
//! So an affected Alacritty inside tmux 3.3 or newer answers nothing, keeps `REPORT_EVENT_TYPES` and still double-submits Enter.
//! Downgrading everything that answers nothing would cost far more healthy sessions than that slice.
//!
//! What losing `REPORT_EVENT_TYPES` costs an affected session:
//!
//! - Voice hold-to-talk degrades to a tap toggle (`voice_chord_action`; the `voice_capture_mode` setting hides its `hold` choice).
//! - `is_link_modifier_for_key`'s non-macOS Ctrl-release case (`xai-grok-pager` `src/app/agent_view/mod.rs`) never fires.
//!   So link-hover clears on the next non-Ctrl key instead of when Ctrl lifts.
//! - `KeyEventKind::Repeat` disappears: held keys arrive as repeated `Press`, as on every non-KKP terminal.
//!   But `is_pasteable_key_event` (`xai-grok-pager` `src/app/event_loop.rs`) excludes `Repeat` on purpose.
//!   So auto-repeat counts toward paste coalescing again.

use std::sync::atomic::{AtomicU8, Ordering};

use crossterm::event::KeyboardEnhancementFlags;

/// Highest packed library version that emits a duplicate legacy release for Backspace/Tab/Enter/Escape.
/// Crossterm reads that as a second Press, so Enter submits twice. Threshold is the 0.14.0 library (`2401`); it only gets retired.
/// Git builds strip `-dev` before packing, so a pre-fix `0.24.2-dev` reports `2402` and escapes this gate.
pub const ALACRITTY_BROKEN_EVENT_TYPES_MAX_PACKED: u32 = 2401;

/// How `[ui].kitty_keyboard` treats an unidentified terminal.
/// `Auto` keeps today's skip, so startup does not wait on a probe.
/// `Query` asks that terminal whether it speaks the Kitty keyboard protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KittyKeyboardMode {
    /// Skip an unidentified terminal. No probe.
    #[default]
    Auto,
    /// Drop only the unidentified-terminal skip and run the probe.
    Query,
}

impl KittyKeyboardMode {
    /// `query` is the only opt-in. Anything else, including unset, stays `Auto`.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) if value.eq_ignore_ascii_case("query") => Self::Query,
            _ => Self::Auto,
        }
    }
}

/// Lift the unidentified-terminal skip when the user asked to query.
/// Every other reason (VTE, screen, old tmux, Zed, Windows Terminal) still skips.
pub fn effective_kitty_skip_reason(
    detected: Option<&'static str>,
    mode: KittyKeyboardMode,
) -> Option<&'static str> {
    if mode == KittyKeyboardMode::Query && detected == Some("unknown_no_multiplexer") {
        None
    } else {
        detected
    }
}

/// Skip reason for the startup push.
/// A remaining detected reason wins and the probe is not called.
/// With no reason, `probe` returning `Some(true)` pushes flags; anything else skips as `unsupported`.
pub fn kitty_push_skip_reason(
    detected: Option<&'static str>,
    mode: KittyKeyboardMode,
    probe: impl FnOnce() -> Option<bool>,
) -> Option<&'static str> {
    if let Some(reason) = effective_kitty_skip_reason(detected, mode) {
        return Some(reason);
    }
    match probe() {
        Some(true) => None,
        _ => Some("unsupported"),
    }
}

/// Empty means push nothing. Unknown version never downgrades — DA2 is often skipped, and only a positively identified broken version pays the cost.
/// No brand check: [`super::da2`]'s gate admits Alacritty alone, so widening that gate widens this one.
pub fn negotiated_kitty_flags(
    skip_reason: Option<&str>,
    da2_packed: Option<u32>,
) -> KeyboardEnhancementFlags {
    if skip_reason.is_some() {
        return KeyboardEnhancementFlags::empty();
    }
    // Alternate keys carry the layout's shifted glyph (`1` → `!`, QWERTZ `7` → `/`).
    // Without that field the protocol reports the unshifted key, and Shift+digit inserts the digit.
    let mut flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
    let mis_encodes_releases =
        da2_packed.is_some_and(|packed| packed <= ALACRITTY_BROKEN_EVENT_TYPES_MAX_PACKED);
    if !mis_encodes_releases {
        flags |= KeyboardEnhancementFlags::REPORT_EVENT_TYPES;
    }
    flags
}

/// Bits actually pushed (`0` is empty), not a classification, so the predicates cannot drift.
/// Relaxed: no other memory is published, and readers are ordered after `init_terminal`.
static PUSHED_KITTY_FLAGS: AtomicU8 = AtomicU8::new(0);

/// The exact flag set `init_terminal` pushed; the suspend/resume path re-pushes this verbatim so the two can never drift.
pub fn pushed_kitty_flags() -> KeyboardEnhancementFlags {
    KeyboardEnhancementFlags::from_bits_truncate(PUSHED_KITTY_FLAGS.load(Ordering::Relaxed))
}

pub fn set_pushed_kitty_flags(flags: KeyboardEnhancementFlags) {
    PUSHED_KITTY_FLAGS.store(flags.bits(), Ordering::Relaxed);
}

/// True only if flags were actually pushed. False means modified keys arrive as legacy bytes.
/// This is not "key releases arrive"; use [`kitty_releases_reported`].
pub fn kitty_flags_pushed() -> bool {
    !pushed_kitty_flags().is_empty()
}

/// Whether the terminal reports key *release* events.
/// Every feature that waits for a release (hold-to-talk, modifier-lift tracking) must gate on this, not on [`kitty_flags_pushed`].
/// The two differ on Alacritty 0.14.x and older.
pub fn kitty_releases_reported() -> bool {
    pushed_kitty_flags().contains(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
}

/// Whether the version workaround engaged: pushed, but without `REPORT_EVENT_TYPES`.
/// Not `!kitty_releases_reported()`, which is also true when nothing was pushed at all.
pub fn kitty_event_types_withheld() -> bool {
    let flags = pushed_kitty_flags();
    !flags.is_empty() && !flags.contains(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
}

/// Clears the record as it reads, so concurrent teardown paths cannot both pop.
pub fn take_kitty_flags_pushed() -> bool {
    PUSHED_KITTY_FLAGS.swap(0, Ordering::Relaxed) != 0
}

/// Crossterm's probe waits two seconds when the terminal sends nothing.
/// A living terminal answers `CSI ? u`, or the primary DA sent with it, well inside this.
/// Silence is "unsupported", same as a timed-out crossterm poll, without holding startup for the rest of that window.
pub const KEYBOARD_QUERY_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

#[cfg(unix)]
const KEYBOARD_QUERY: &[u8] = b"\x1b[?u\x1b[c";

/// The primary-DA reply usually follows the flags reply in the same burst.
/// Long enough to catch that burst, short enough that a missing DA does not bring the two-second wait back.
#[cfg(unix)]
const DA_FOLLOWUP: std::time::Duration = std::time::Duration::from_millis(20);

/// Ask the terminal whether it speaks the Kitty keyboard protocol.
/// `Some(true)` is a flags reply (`CSI ? <digits> u`). `Some(false)` is a primary DA reply and no flags reply.
/// `None` is silence, a failed write, or a non-Unix host.
pub fn query_keyboard_enhancement() -> Option<bool> {
    #[cfg(unix)]
    {
        if !super::probe::write_query(KEYBOARD_QUERY) {
            return None;
        }
        let reply = super::probe::read_tty_reply(KEYBOARD_QUERY_TIMEOUT, |buf, _| {
            keyboard_query_verdict(buf).is_some()
        });
        let verdict = reply.as_deref().and_then(keyboard_query_verdict);
        if verdict == Some(true) {
            let _ = super::probe::read_tty_reply(DA_FOLLOWUP, |buf, _| da1_reply(buf));
        }
        verdict
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// `Some(true)` when `buf` holds a flags reply. `Some(false)` when it holds only a primary DA reply.
/// `None` when neither reply is complete.
pub fn keyboard_query_verdict(buf: &[u8]) -> Option<bool> {
    if kitty_flags_reply(buf) {
        Some(true)
    } else if da1_reply(buf) {
        Some(false)
    } else {
        None
    }
}

/// `CSI ? <digits> u`. The query we send is `CSI ? u` with no digits, so that form is not a reply.
fn kitty_flags_reply(buf: &[u8]) -> bool {
    buf.windows(3).enumerate().any(|(i, window)| {
        if window != b"\x1b[?" {
            return false;
        }
        let rest = buf.get(i + 3..).unwrap_or(b"");
        let digits = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
        digits > 0 && rest.get(digits) == Some(&b'u')
    })
}

/// Primary device attributes: `CSI ? <params> c`, params being digits and semicolons.
fn da1_reply(buf: &[u8]) -> bool {
    buf.windows(3).enumerate().any(|(i, window)| {
        if window != b"\x1b[?" {
            return false;
        }
        let rest = buf.get(i + 3..).unwrap_or(b"");
        let params = rest
            .iter()
            .take_while(|byte| byte.is_ascii_digit() || **byte == b';')
            .count();
        params > 0 && rest.get(params) == Some(&b'c')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISAMBIGUATE: KeyboardEnhancementFlags =
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES;
    const ALTERNATES: KeyboardEnhancementFlags = KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
    const EVENT_TYPES: KeyboardEnhancementFlags = KeyboardEnhancementFlags::REPORT_EVENT_TYPES;

    /// The exact boundary a careless `<`/`<=` edit breaks: `alacritty_terminal` 0.24.1 is Alacritty 0.14.0 (broken), 0.24.2 is 0.15.0 (fixed).
    #[test]
    fn downgrade_boundary_is_the_last_broken_library_version() {
        // Non-empty either side: a downgrade is still a push, teardown owes a pop.
        assert_eq!(
            negotiated_kitty_flags(None, Some(2401)),
            DISAMBIGUATE | ALTERNATES
        );
        assert_eq!(
            negotiated_kitty_flags(None, Some(2402)),
            DISAMBIGUATE | ALTERNATES | EVENT_TYPES
        );
    }

    /// DA2 is skipped under multiplexers and off unix, so "no answer" is the common case and must not cost a healthy terminal its release events.
    #[test]
    fn absent_version_does_not_downgrade() {
        assert_eq!(
            negotiated_kitty_flags(None, None),
            DISAMBIGUATE | ALTERNATES | EVENT_TYPES
        );
    }

    /// `[ui] kitty_keyboard = "query"` asks an unidentified terminal. The default does not.
    /// A yes from the probe negotiates flags, which is what lets Shift+Enter arrive as a newline.
    /// A known-bad terminal is not asked.
    #[test]
    fn query_mode_lets_an_unidentified_terminal_negotiate_shift_enter() {
        let detected = Some("unknown_no_multiplexer");
        let mut probes = 0;
        let skipped = kitty_push_skip_reason(detected, KittyKeyboardMode::Auto, || {
            probes += 1;
            Some(true)
        });
        assert_eq!(skipped, detected);
        assert_eq!(probes, 0);
        assert!(
            negotiated_kitty_flags(skipped, None).is_empty(),
            "auto must not push flags for an unidentified terminal"
        );

        let negotiated = kitty_push_skip_reason(detected, KittyKeyboardMode::Query, || {
            probes += 1;
            Some(true)
        });
        assert_eq!(negotiated, None);
        assert_eq!(probes, 1);
        assert!(
            !negotiated_kitty_flags(negotiated, None).is_empty(),
            "a positive probe must push the flags Shift+Enter needs"
        );

        let declined = kitty_push_skip_reason(detected, KittyKeyboardMode::Query, || Some(false));
        assert_eq!(declined, Some("unsupported"));
        assert!(negotiated_kitty_flags(declined, None).is_empty());

        let mut probes = 0;
        let vte = kitty_push_skip_reason(Some("vte"), KittyKeyboardMode::Query, || {
            probes += 1;
            Some(true)
        });
        assert_eq!(vte, Some("vte"));
        assert_eq!(probes, 0);
    }

    #[test]
    fn kitty_keyboard_query_parses_only_that_word() {
        assert_eq!(
            KittyKeyboardMode::parse(Some("query")),
            KittyKeyboardMode::Query
        );
        assert_eq!(
            KittyKeyboardMode::parse(Some(" Query ")),
            KittyKeyboardMode::Query
        );
        assert_eq!(KittyKeyboardMode::parse(None), KittyKeyboardMode::Auto);
        assert_eq!(
            KittyKeyboardMode::parse(Some("off")),
            KittyKeyboardMode::Auto
        );
        assert_eq!(KittyKeyboardMode::parse(Some("")), KittyKeyboardMode::Auto);
    }

    #[test]
    fn keyboard_query_verdict_reads_flags_and_primary_da() {
        assert_eq!(keyboard_query_verdict(b"\x1b[?15u"), Some(true));
        assert_eq!(keyboard_query_verdict(b"hi\x1b[?1u"), Some(true));
        // Flags win when the primary DA is already in the same buffer.
        assert_eq!(keyboard_query_verdict(b"\x1b[?15u\x1b[?62;1;6c"), Some(true));
        assert_eq!(keyboard_query_verdict(b"\x1b[?62;1;6c"), Some(false));
        // The query itself, a bare DA request, and silence are not replies.
        assert_eq!(keyboard_query_verdict(b"\x1b[?u"), None);
        assert_eq!(keyboard_query_verdict(b"\x1b[c"), None);
        assert_eq!(keyboard_query_verdict(b""), None);
        assert_eq!(keyboard_query_verdict(b"\x1b[?15"), None);
    }

    /// A capable terminal with no skip reason still probes. Silence is unsupported, and the wait is the query budget, not crossterm's two seconds.
    #[test]
    fn unanswered_keyboard_query_is_under_two_seconds() {
        assert!(KEYBOARD_QUERY_TIMEOUT <= std::time::Duration::from_millis(500));
        assert!(KEYBOARD_QUERY_TIMEOUT < std::time::Duration::from_secs(2));
        let skipped = kitty_push_skip_reason(None, KittyKeyboardMode::Auto, || None);
        assert_eq!(skipped, Some("unsupported"));
        assert!(negotiated_kitty_flags(skipped, None).is_empty());
    }

    /// A skip reason outranks any version, so teardown owes no pop.
    #[test]
    fn a_skip_reason_pushes_nothing() {
        for packed in [None, Some(2401), Some(2402)] {
            assert_eq!(
                negotiated_kitty_flags(Some("vscode"), packed),
                KeyboardEnhancementFlags::empty(),
                "da2_packed={packed:?}"
            );
        }
    }
}
