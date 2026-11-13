//! Plan-nudge trigger: detects planning keywords typed into the prompt so the
//! pager can hint that Shift+Tab cycles into plan mode first.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::EphemeralTip;
use crate::theme::Theme;

/// Ephemeral-tip dedup key for the plan-mode nudge.
pub(crate) const PLAN_NUDGE_KEY: &str = "plan_nudge";

/// Key into the per-session in-memory seen-count map
/// (`AppView::tip_seen_counts`) for the plan nudge. Not persisted to disk.
pub(crate) const PLAN_NUDGE_SEEN_KEY: &str = "plan_nudge_shown_count";

/// The tip stops showing after this many shows within a single session.
const PLAN_NUDGE_SEEN_CAP: u32 = 3;

/// Allowlist of planning intents (ASCII lowercase), kept tight to avoid false positives.
/// Matched as whole words (see [`prompt_mentions_planning`]) so "explain"/"explanation"/"planet" never trip the "plan" entry.
const PLANNING_KEYWORDS: &[&str] = &[
    "plan",
    "planning",
    "design",
    "architect",
    "step by step",
    "break this down",
    "lay out",
    "approach",
    "strategy",
];

/// Plan-mode chord for the tip copy: always `Shift+Tab`.
/// Derived from the real `CycleMode` binding (not a literal), so it can't drift.
/// `shift_tab_keys()[0]` is one of the encodings [`crate::input::key::is_shift_tab`] accepts.
fn plan_chord_label() -> String {
    crate::input::key::shift_tab_keys()[0].display()
}

/// Build the "Planning? Check out plan mode via {chord}" tip, capped at [`PLAN_NUDGE_SEEN_CAP`] shows per session (in-memory).
pub fn plan_nudge_tip() -> EphemeralTip {
    let theme = Theme::current();
    let dim = Style::default().fg(theme.gray);
    // The key chord is styled like the shortcuts bar (bold secondary on dim text)
    let chord = Style::default()
        .fg(theme.text_secondary)
        .add_modifier(Modifier::BOLD);
    EphemeralTip::new(
        PLAN_NUDGE_KEY,
        Line::from(vec![
            Span::styled("Planning? Check out plan mode via ", dim),
            Span::styled(plan_chord_label(), chord),
        ]),
    )
    .with_session_seen_cap(PLAN_NUDGE_SEEN_KEY, PLAN_NUDGE_SEEN_CAP)
}

/// Whether `text` mentions a planning intent from the tight [`PLANNING_KEYWORDS`] allowlist.
/// Case-insensitive and matched on whole-word boundaries so near neighbours ("explain", "explanation", "planet", "redesign") never match.
/// Non-allocating full scan. The prompt does not run this on every character of a long draft; `PlanningMentionCache` rechecks only the edited span.
pub fn prompt_mentions_planning(text: &str) -> bool {
    first_planning_hit(text, 0, usize::MAX).is_some()
}

fn max_planning_keyword_len() -> usize {
    PLANNING_KEYWORDS.iter().map(|kw| kw.len()).max().unwrap_or(0)
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b >= 0x80
}

fn boundary_before(hay: &[u8], start: usize) -> bool {
    start == 0
        || start
            .checked_sub(1)
            .and_then(|index| hay.get(index))
            .is_none_or(|b| !is_word_byte(*b))
}

fn boundary_after(hay: &[u8], end: usize) -> bool {
    hay.get(end).is_none_or(|b| !is_word_byte(*b))
}

/// Whole-word, case-insensitive keyword at `start` in the full buffer.
/// Boundaries are read from `hay`, never from a sliced window: a slice edge is not a word boundary.
fn keyword_at(hay: &[u8], start: usize, needle: &[u8]) -> bool {
    if needle.is_empty() {
        return false;
    }
    let Some(end) = start.checked_add(needle.len()) else {
        return false;
    };
    let Some(window) = hay.get(start..end) else {
        return false;
    };
    window
        .iter()
        .zip(needle)
        .all(|(h, n)| h.to_ascii_lowercase() == *n)
        && boundary_before(hay, start)
        && boundary_after(hay, end)
}

/// Earliest allowlist hit whose start byte lies in `start_lo..start_hi` (exclusive end).
fn first_planning_hit(text: &str, start_lo: usize, start_hi: usize) -> Option<std::ops::Range<usize>> {
    let hay = text.as_bytes();
    let mut best: Option<std::ops::Range<usize>> = None;
    for kw in PLANNING_KEYWORDS {
        let needle = kw.as_bytes();
        if needle.is_empty() || hay.len() < needle.len() {
            continue;
        }
        let last = hay.len() - needle.len();
        if start_lo > last {
            continue;
        }
        let hi = start_hi.min(last.saturating_add(1));
        if start_lo >= hi {
            continue;
        }
        for start in start_lo..hi {
            if !keyword_at(hay, start, needle) {
                continue;
            }
            let range = start..start + needle.len();
            let earlier = best.as_ref().is_none_or(|have| range.start < have.start);
            if earlier {
                best = Some(range);
            }
            break;
        }
    }
    best
}

fn keyword_range_is_hit(text: &str, range: &std::ops::Range<usize>) -> bool {
    let Some(slice) = text.get(range.start..range.end) else {
        return false;
    };
    let hay = text.as_bytes();
    if !boundary_before(hay, range.start) || !boundary_after(hay, range.end) {
        return false;
    }
    PLANNING_KEYWORDS.iter().any(|kw| {
        kw.len() == slice.len()
            && slice
                .bytes()
                .zip(kw.bytes())
                .all(|(byte, needle)| byte.to_ascii_lowercase() == needle)
    })
}

/// One contiguous replace: `removed` bytes at `old_start` in the pre-edit text became `inserted` bytes.
struct LocalEdit {
    old_start: usize,
    removed: usize,
    inserted: usize,
}

fn classify_local_edit(
    cursor_before: usize,
    selection_before: Option<std::ops::Range<usize>>,
    len_before: usize,
    cursor_after: usize,
    len_after: usize,
) -> Option<LocalEdit> {
    if let Some(sel) = selection_before {
        if sel.end > len_before {
            return None;
        }
        let removed = sel.end.checked_sub(sel.start)?;
        let inserted = len_after.checked_add(removed)?.checked_sub(len_before)?;
        return Some(LocalEdit {
            old_start: sel.start,
            removed,
            inserted,
        });
    }
    if len_after >= len_before {
        let inserted = len_after - len_before;
        if cursor_before <= len_before && cursor_after == cursor_before.saturating_add(inserted) {
            return Some(LocalEdit {
                old_start: cursor_before,
                removed: 0,
                inserted,
            });
        }
        return None;
    }
    let removed = len_before - len_after;
    // Backspace: the cursor sits on the join, which was just after the deleted bytes.
    if cursor_before == cursor_after.saturating_add(removed) && cursor_before <= len_before {
        return Some(LocalEdit {
            old_start: cursor_after,
            removed,
            inserted: 0,
        });
    }
    // Forward delete: the cursor stays on the join.
    if cursor_after == cursor_before && cursor_before.saturating_add(removed) <= len_before {
        return Some(LocalEdit {
            old_start: cursor_before,
            removed,
            inserted: 0,
        });
    }
    None
}

fn shift_hit(hit: &std::ops::Range<usize>, edit: &LocalEdit) -> Option<std::ops::Range<usize>> {
    let old_end = edit.old_start.saturating_add(edit.removed);
    if hit.end <= edit.old_start {
        return Some(hit.clone());
    }
    if hit.start >= old_end {
        let delta = edit.inserted as isize - edit.removed as isize;
        let start = hit.start as isize + delta;
        let end = hit.end as isize + delta;
        if start >= 0 && end >= start {
            return Some((start as usize)..(end as usize));
        }
    }
    None
}

/// Cached answer of [`prompt_mentions_planning`] for one prompt buffer.
///
/// Keystroke paste delivers one scalar per event. Scanning the whole draft on each event freezes the
/// prompt once the draft is long. A new match has to overlap the inserted or deleted bytes (or the
/// single boundary byte next to them), so a local edit searches only that window. A match the edit
/// does not touch stays valid with no scan.
#[derive(Debug, Default)]
pub(crate) struct PlanningMentionCache {
    revision: Option<xai_ratatui_textarea::BufferContentRevision>,
    /// Byte range of one whole-word hit. `None` with `revision` set means the draft was scanned and has none.
    hit: Option<std::ops::Range<usize>>,
    full_scans: u64,
    bytes_scanned: u64,
}

impl PlanningMentionCache {
    pub(crate) fn mentions(&self) -> bool {
        self.hit.is_some()
    }

    /// Adopt `revision` when the cached answer is stale. A cold cache scans the whole draft once.
    pub(crate) fn ensure(
        &mut self,
        text: &str,
        revision: xai_ratatui_textarea::BufferContentRevision,
    ) {
        if self.revision == Some(revision) {
            return;
        }
        self.rescan(text, revision);
    }

    /// Update the cache after a textarea edit that moved `revision_before` to `revision`.
    pub(crate) fn note_after_edit(
        &mut self,
        text: &str,
        revision: xai_ratatui_textarea::BufferContentRevision,
        revision_before: xai_ratatui_textarea::BufferContentRevision,
        cursor_before: usize,
        selection_before: Option<std::ops::Range<usize>>,
        len_before: usize,
        cursor_after: usize,
    ) {
        if self.revision == Some(revision) {
            return;
        }
        if self.revision != Some(revision_before) {
            self.rescan(text, revision);
            return;
        }
        let Some(edit) = classify_local_edit(
            cursor_before,
            selection_before,
            len_before,
            cursor_after,
            text.len(),
        ) else {
            self.rescan(text, revision);
            return;
        };
        // Same length and an unmoved cursor is not a local insert. The bytes may have changed elsewhere.
        if edit.removed == 0 && edit.inserted == 0 {
            self.rescan(text, revision);
            return;
        }
        self.apply_local(text, revision, edit);
    }

    #[cfg(test)]
    pub(crate) fn scan_cost(&self) -> (u64, u64) {
        (self.full_scans, self.bytes_scanned)
    }

    fn rescan(&mut self, text: &str, revision: xai_ratatui_textarea::BufferContentRevision) {
        self.full_scans = self.full_scans.saturating_add(1);
        self.bytes_scanned = self.bytes_scanned.saturating_add(text.len() as u64);
        self.hit = first_planning_hit(text, 0, usize::MAX);
        self.revision = Some(revision);
    }

    fn apply_local(
        &mut self,
        text: &str,
        revision: xai_ratatui_textarea::BufferContentRevision,
        edit: LocalEdit,
    ) {
        let was_present = self.hit.is_some();
        if let Some(hit) = self.hit.clone()
            && let Some(shifted) = shift_hit(&hit, &edit)
            && keyword_range_is_hit(text, &shifted)
        {
            self.hit = Some(shifted);
            self.revision = Some(revision);
            return;
        }
        let max_kw = max_planning_keyword_len();
        let new_start = edit.old_start.min(text.len());
        let new_end = new_start.saturating_add(edit.inserted).min(text.len());
        let lo = new_start.saturating_sub(max_kw.saturating_sub(1));
        // One past the inserted bytes: a keyword can start on the first byte after the edit when only its boundary changed.
        let hi = new_end.saturating_add(1);
        let span_end = hi
            .saturating_add(max_kw)
            .min(text.len().saturating_add(1));
        self.bytes_scanned = self
            .bytes_scanned
            .saturating_add(span_end.saturating_sub(lo.min(text.len())) as u64);
        if let Some(found) = first_planning_hit(text, lo, hi) {
            self.hit = Some(found);
            self.revision = Some(revision);
            return;
        }
        if !was_present {
            self.hit = None;
            self.revision = Some(revision);
            return;
        }
        // The stored hit was destroyed and the edit window has no other keyword. One may still sit elsewhere.
        self.rescan(text, revision);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_planning_keywords_as_whole_words() {
        assert!(prompt_mentions_planning("let's plan the refactor"));
        assert!(prompt_mentions_planning("Plan it first"));
        assert!(prompt_mentions_planning("some planning before we code"));
        assert!(prompt_mentions_planning("design the system"));
        assert!(prompt_mentions_planning("architect this module"));
        assert!(prompt_mentions_planning(
            "walk me through this step by step"
        ));
        assert!(prompt_mentions_planning("break this down for me"));
        assert!(prompt_mentions_planning("lay out the migration"));
        assert!(prompt_mentions_planning("what's your approach?"));
        assert!(prompt_mentions_planning("pick a strategy"));
    }

    #[test]
    fn does_not_match_near_neighbours() {
        // The headline false positive: "explain" must NOT match "plan".
        assert!(!prompt_mentions_planning("explain this code"));
        assert!(!prompt_mentions_planning("can you explain the explanation"));
        assert!(!prompt_mentions_planning("the planet is round"));
        assert!(!prompt_mentions_planning("redesigned the airplane wing"));
        assert!(!prompt_mentions_planning("fix the bug in main.rs"));
        assert!(!prompt_mentions_planning(""));
    }

    #[test]
    fn plan_nudge_builder_applies_seen_gating() {
        // The fact to pin is that the builder opts into the per-session seen gate at all; asserting the exact key and cap would restate the constants
        assert_eq!(
            plan_nudge_tip().session_seen.map(|(key, _cap)| key),
            Some(PLAN_NUDGE_SEEN_KEY)
        );
    }

    #[test]
    fn plan_nudge_chord_is_shift_tab() {
        // Always Shift+Tab; derived from the real binding so it can't drift
        assert_eq!(plan_chord_label(), "Shift+Tab");
        // The advertised chord is genuinely one is_shift_tab accepts.
        assert!(crate::input::key::is_shift_tab(
            &crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::BackTab,
                crossterm::event::KeyModifiers::NONE,
            )
        ));
    }
}
