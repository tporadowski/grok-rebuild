//! Display-column fit for CLI tables.
//!
//! Shell cannot depend on `xai-grok-pager-render` (the pager depends on shell).
//! The cut matches that crate's `truncate_to_width`: wide characters count as two
//! columns, emoji presentation counts as the terminal draws it, and a cut ends
//! with one-column `…`.

use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Columns one grapheme cluster takes on the terminal.
///
/// `⚠️` and `#️⃣` are a base plus a variation selector (and a keycap mark). Per-character width
/// counts those as one column; the terminal draws them as two.
fn cluster_columns(cluster: &str) -> usize {
    let per_char: usize = cluster
        .chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
        .sum();
    if !cluster.contains('\u{FE0F}') {
        return per_char;
    }
    UnicodeWidthStr::width(cluster).max(per_char)
}

/// Terminal columns of `s`, counting emoji presentation and keycap sequences the way the terminal draws them.
pub(crate) fn display_columns(s: &str) -> usize {
    s.graphemes(true)
        .map(cluster_columns)
        .fold(0usize, usize::saturating_add)
}

/// Byte offset at which display width would exceed `max_width`, or `s.len()`.
fn byte_offset_at_width(s: &str, max_width: usize) -> usize {
    let mut width = 0usize;
    for (idx, cluster) in s.grapheme_indices(true) {
        let columns = cluster_columns(cluster);
        if width.saturating_add(columns) > max_width {
            return idx;
        }
        width = width.saturating_add(columns);
    }
    s.len()
}

/// Truncate to at most `max_width` display columns, ending with `…` when cut.
pub(crate) fn truncate_to_width(s: &str, max_width: usize) -> Cow<'_, str> {
    if byte_offset_at_width(s, max_width) == s.len() {
        return Cow::Borrowed(s);
    }
    if max_width == 0 {
        return Cow::Borrowed("");
    }
    let end = byte_offset_at_width(s, max_width - 1);
    let Some(prefix) = s.get(..end) else {
        return Cow::Borrowed(s);
    };
    Cow::Owned(format!("{prefix}…"))
}

/// Left-align `s` in `width` display columns.
pub(crate) fn pad_to_width(s: &str, width: usize) -> String {
    let pad = width.saturating_sub(display_columns(s));
    let mut out = String::with_capacity(s.len() + pad);
    out.push_str(s);
    out.extend(std::iter::repeat_n(' ', pad));
    out
}

/// Truncate, then pad, so a cell occupies exactly `width` columns when `width` is positive
/// and the text had to be cut or was short.
pub(crate) fn fit_cell(s: &str, width: usize) -> String {
    pad_to_width(truncate_to_width(s, width).as_ref(), width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_cell_keeps_wide_text_inside_the_column() {
        assert_eq!(fit_cell("ab", 4), "ab  ");
        assert_eq!(fit_cell("日本", 6), "日本  ");
        assert_eq!(truncate_to_width("日本語ラベル", 5).as_ref(), "日本…");
        let wide = fit_cell(&"日本語".repeat(20), 40);
        // Padding follows a cut that misses the column edge, so `…` is not the last byte.
        assert!(wide.trim_end().ends_with('…'), "{wide}");
        assert_eq!(display_columns(&wide), 40);
        let warning = fit_cell(&format!("{}abcd", "\u{26A0}\u{FE0F}".repeat(10)), 5);
        assert!(warning.trim_end().ends_with('…'), "{warning}");
        assert!(display_columns(&warning) <= 5, "{warning:?}");
    }
}
