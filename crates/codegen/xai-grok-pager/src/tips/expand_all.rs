//! Tip shown after repeated thought folds while expand-all is still on.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::EphemeralTip;
use crate::theme::Theme;

pub(crate) const EXPAND_ALL_TIP_KEY: &str = "expand_all_tip";

pub(crate) const EXPAND_ALL_TIP_SEEN_KEY: &str = "expand_all_tip_shown_count";

const EXPAND_ALL_TIP_SEEN_CAP: u32 = 3;

pub(crate) const EXPAND_ALL_TIP_TICKS: u16 = 600;

/// How many thought collapses under expand-all arm the hint. One fold is ordinary; the next is "keep collapsing".
pub(crate) const EXPAND_ALL_COLLAPSE_HINT_AFTER: u8 = 2;

/// `chord` is the live binding for toggling every thinking block (Ctrl+E unless rebound).
pub fn expand_all_tip(chord: &str) -> EphemeralTip {
    let theme = Theme::current();
    let dim = super::render::tip_dim(&theme);
    let key_style = Style::default()
        .fg(theme.text_secondary)
        .add_modifier(Modifier::BOLD);
    let chord = chord.to_owned();
    EphemeralTip {
        ticks_remaining: EXPAND_ALL_TIP_TICKS,
        ..EphemeralTip::new(
            EXPAND_ALL_TIP_KEY,
            Line::from(vec![
                Span::styled("Folding thoughts one by one? ", dim),
                Span::styled(chord, key_style),
                Span::styled(" turns expand-all off", dim),
            ]),
        )
        .with_session_seen_cap(EXPAND_ALL_TIP_SEEN_KEY, EXPAND_ALL_TIP_SEEN_CAP)
        .ambient()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_all_tip_names_the_chord_that_turns_it_off() {
        let tip = expand_all_tip("Ctrl+E");
        let text: String = tip.line.spans.iter().map(|span| span.content.as_ref()).collect();
        assert!(
            text.contains("Ctrl+E") && text.contains("turns expand-all off"),
            "got {text:?}"
        );
        assert_eq!(tip.session_seen.map(|(key, _)| key), Some(EXPAND_ALL_TIP_SEEN_KEY));
        assert!(tip.ambient);
    }
}
