//! Tab width for painted markdown.
//!
//! The stored source keeps `\t` so copying a reply or a code block can return the original text.
//! Ratatui drops a cluster that holds a tab, so a span expands tabs to spaces when it is painted.

use std::borrow::Cow;
use std::sync::atomic::{AtomicU8, Ordering};

static PAINT_TAB_WIDTH: AtomicU8 = AtomicU8::new(4);

/// Spaces painted for one `\t`. Zero leaves the tab in the span.
pub fn set_paint_tab_width(width: u8) {
    PAINT_TAB_WIDTH.store(width, Ordering::Relaxed);
}

pub(crate) fn paint_tab_width() -> u8 {
    PAINT_TAB_WIDTH.load(Ordering::Relaxed)
}

/// Replace each tab with [`paint_tab_width`] spaces.
/// Width zero, or a string with no tab, borrows `text`.
pub(crate) fn expand_paint_tabs(text: &str) -> Cow<'_, str> {
    let width = paint_tab_width();
    if width == 0 || !text.contains('\t') {
        return Cow::Borrowed(text);
    }
    Cow::Owned(text.replace('\t', &" ".repeat(usize::from(width))))
}
