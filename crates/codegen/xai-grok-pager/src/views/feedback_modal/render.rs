//! Feedback modal layout and rendering.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::{
    CANCEL_SHORTCUT_ID, DraftSubmitTerminal, DraftsState, FEEDBACK_TABS, FeedbackModalRender,
    FeedbackModalState, FeedbackModalStep, FeedbackTab, FeedbackTraceChoice,
};
use crate::theme::Theme;
use crate::views::modal_window::{self, ModalSizing, ModalWindowConfig, Shortcut};
use crate::views::prompt_widget::PromptStyle;

/// One-sentence status and confirm copy loses only a trailing period.
fn shown_modal_sentence(text: &str) -> &str {
    crate::views::modal::drop_one_sentence_period(text)
}

/// How many rows `text` occupies when wrapped to `width` columns.
fn status_row_count(text: &str, width: u16) -> u16 {
    if width == 0 || text.is_empty() {
        return 0;
    }
    let line = ratatui::text::Line::from(text);
    let rows = crate::render::wrapping::word_wrap_line(&line, usize::from(width)).len();
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// Paint `text` wrapped to `area`, and return the rows used. Nothing is drawn past `area`.
fn paint_wrapped_status(
    buf: &mut Buffer,
    area: Rect,
    text: &str,
    style: ratatui::style::Style,
) -> u16 {
    if area.width == 0 || area.height == 0 || text.is_empty() {
        return 0;
    }
    let line = ratatui::text::Line::styled(text, style);
    let wrapped = crate::render::wrapping::word_wrap_line(&line, usize::from(area.width));
    let mut painted = 0u16;
    for wrapped_line in wrapped {
        let y = area.y.saturating_add(painted);
        if y >= area.bottom() {
            break;
        }
        buf.set_line(area.x, y, &wrapped_line, area.width);
        painted = painted.saturating_add(1);
    }
    painted
}

/// List label for one draft. A stored title wins. A blank title falls back to
/// the first non-blank details line, which is how a free-text predraft reads.
fn draft_row_preview<'a>(title: &'a str, details: &'a str) -> &'a str {
    let title = title.trim();
    if !title.is_empty() {
        return title;
    }
    details
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

/// Classification, then the title. The title is last, so a narrow dialog used to clip it.
fn draft_row_parts(draft: &super::FeedbackDraft) -> (String, String) {
    let preview = draft_row_preview(&draft.title, &draft.details).to_string();
    let failure = draft
        .failure_mode
        .map(|value| format!(" · {}", value.label()))
        .unwrap_or_default();
    let r#type = draft.r#type.map_or("Unclassified", |value| value.label());
    let task = draft.task_category.map_or("Other", |value| value.label());
    (format!("{type} · {task}{failure}"), preview)
}

/// One drafts-list row, same characters `draft_row_line` paints.
fn draft_row_text(draft: &super::FeedbackDraft, selected: bool) -> String {
    let marker = if selected { "❯" } else { " " };
    let (meta, preview) = draft_row_parts(draft);
    format!("{marker} {meta} · {preview}")
}

/// Metadata stays secondary. The title stays primary and bold. The band, not a recolor, is the selection.
fn draft_row_line(
    draft: &super::FeedbackDraft,
    selected: bool,
    theme: &Theme,
) -> ratatui::text::Line<'static> {
    let marker = if selected { "❯ " } else { "  " };
    let (meta, preview) = draft_row_parts(draft);
    let secondary = ratatui::style::Style::default().fg(theme.text_secondary);
    let title = ratatui::style::Style::default()
        .fg(theme.text_primary)
        .add_modifier(ratatui::style::Modifier::BOLD);
    ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(marker.to_string(), secondary),
        ratatui::text::Span::styled(meta, secondary),
        ratatui::text::Span::styled(" · ".to_string(), secondary),
        ratatui::text::Span::styled(preview, title),
    ])
}

fn draft_row_height(draft: &super::FeedbackDraft, width: u16) -> usize {
    usize::from(status_row_count(&draft_row_text(draft, false), width)).max(1)
}

fn row_span(heights: &[usize], start: usize, end: usize) -> usize {
    heights
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .fold(0usize, |sum, height| sum.saturating_add((*height).max(1)))
}

/// Scroll the drafts list by wrapped row height so the selected draft stays fully in view.
fn fit_draft_viewport(heights: &[usize], selected: usize, previous: usize, rows: usize) -> usize {
    if heights.is_empty() || rows == 0 {
        return 0;
    }
    let last = heights.len().saturating_sub(1);
    let selected = selected.min(last);
    let mut start = previous.min(last);
    if selected < start {
        start = selected;
    }
    while start > 0 && row_span(heights, start, heights.len()) < rows {
        start -= 1;
    }
    while start < selected && row_span(heights, start, selected.saturating_add(1)) > rows {
        start += 1;
    }
    start
}

/// Wrap a confirm question and its answer. The answer stays on screen when the question needs more than one row.
fn paint_confirm(
    buf: &mut Buffer,
    content: Rect,
    prompt: &str,
    answer: &str,
    prompt_style: ratatui::style::Style,
    answer_style: ratatui::style::Style,
) {
    if content.width == 0 || content.height == 0 {
        return;
    }
    let prompt = shown_modal_sentence(prompt);
    let prompt_rows = status_row_count(prompt, content.width);
    let answer_rows = status_row_count(answer, content.width).max(1);
    let (prompt_keep, gap, answer_keep) =
        if prompt_rows.saturating_add(1).saturating_add(answer_rows) <= content.height {
            (prompt_rows, 1u16, answer_rows)
        } else if prompt_rows.saturating_add(answer_rows) <= content.height {
            (prompt_rows, 0, answer_rows)
        } else {
            let answer_keep = answer_rows.min(content.height);
            (content.height.saturating_sub(answer_keep), 0, answer_keep)
        };
    if prompt_keep > 0 {
        paint_wrapped_status(
            buf,
            Rect {
                height: prompt_keep,
                ..content
            },
            prompt,
            prompt_style,
        );
    }
    if answer_keep > 0 {
        paint_wrapped_status(
            buf,
            Rect {
                x: content.x,
                y: content.y.saturating_add(prompt_keep).saturating_add(gap),
                width: content.width,
                height: answer_keep,
            },
            answer,
            answer_style,
        );
    }
}

impl FeedbackModalState {
    pub fn render(
        &mut self,
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        compact: bool,
    ) -> Option<FeedbackModalRender> {
        self.window.active_tab = self.active_tab().index();
        if self.active_tab() == FeedbackTab::Drafts {
            return self.render_drafts(buf, area, theme, compact);
        }
        let trace_selected = match &self.step {
            FeedbackModalStep::Trace { selected, .. } => Some(*selected),
            _ => None,
        };
        // The order seeded at open; membership is identical to the payload's.
        let fields = self.field_order.clone();
        let shortcuts: &[Shortcut<'static>] = match self.submit_terminal {
            Some(DraftSubmitTerminal::OutcomeUnknown) => Self::UNKNOWN_SUBMIT_SHORTCUTS,
            Some(DraftSubmitTerminal::CleanupFailed) => Self::TERMINAL_SUBMIT_SHORTCUTS,
            None if trace_selected.is_some() => Self::TRACE_SHORTCUTS,
            None if self.enum_picker.is_some() => Self::PICKER_SHORTCUTS,
            None if self.metadata_focus.is_some() => Self::METADATA_SHORTCUTS,
            None if fields.is_empty() => Self::WRITE_SHORTCUTS,
            None => Self::WRITE_SHORTCUTS_WITH_LABELS,
        };
        let mut config = Self::window_config(shortcuts, compact, trace_selected.is_some());
        // A failed status wraps inside the footer, above the shortcuts, instead of running into the border.
        if trace_selected.is_none()
            && let Some(error) = self.error.as_deref()
        {
            let width = modal_window::footer_text_width(area, &config.sizing);
            let status_rows = status_row_count(shown_modal_sentence(error), width);
            let shortcut_rows = modal_window::shortcuts_rows_needed(config.shortcuts, width);
            config.sizing.footer_lines = shortcut_rows.saturating_add(status_rows);
        }
        // Stale label rects must not keep catching clicks after a failed or label-less render.
        self.metadata_row_areas.clear();
        let areas = modal_window::render_modal_window(buf, area, &mut self.window, &config, theme)?;
        if let Some(selected) = trace_selected {
            self.render_trace_step(buf, areas.content, theme, selected);
            // The composer is hidden behind the question, so there is no caret to place.
            return Some(FeedbackModalRender {
                cursor: None,
                post_flush: None,
            });
        }
        let mut content = areas.content;
        // Reserve one full-width row per supplied enum, but only while the composer keeps at
        // least two rows; a too-small render drops the labels, never the draft.
        let label_rows = fields.len() as u16;
        if label_rows > 0 && content.height >= label_rows + 2 {
            self.render_metadata_rows(buf, content, areas.inner_x, theme, &fields);
            content.y += label_rows;
            content.height -= label_rows;
        } else {
            // No rows on screen (absent enums or a too-small render, e.g. after a resize):
            // nothing is focusable, and a lingering focus would only hide the caret.
            self.metadata_focus = None;
        }
        if self.enum_picker.is_some() {
            self.render_enum_picker(buf, content, areas.inner_x, areas.inner_width, theme);
            // The picker's search bar paints its own inverse-video cursor cell; no terminal caret.
            return Some(FeedbackModalRender {
                cursor: None,
                post_flush: None,
            });
        }
        let prompt_result =
            self.composer
                .draw(buf, content, Some(area), &Self::prompt_style(), None, None);
        if let Some(error) = self.error.as_deref() {
            let error = shown_modal_sentence(error);
            let shortcut_rows =
                modal_window::shortcuts_rows_needed(shortcuts, areas.footer.width);
            let status_rows = areas.footer.height.saturating_sub(shortcut_rows);
            if status_rows > 0 {
                paint_wrapped_status(
                    buf,
                    Rect {
                        x: areas.footer.x,
                        y: areas.footer.y,
                        width: areas.footer.width,
                        height: status_rows,
                    },
                    error,
                    ratatui::style::Style::default().fg(theme.accent_error),
                );
            }
        }
        Some(FeedbackModalRender {
            // The terminal caret is the composer-focus signal; a focused label row hides it.
            cursor: if self.metadata_focus.is_some() {
                None
            } else {
                prompt_result.cursor_pos
            },
            post_flush: prompt_result.post_flush_escapes.map(Into::into),
        })
    }

    fn measure_draft_list(
        &self,
        width: u16,
        content_height: u16,
    ) -> Option<(Vec<usize>, usize, usize)> {
        let browse_error = match &self.drafts {
            DraftsState::Browse { error, .. } => error.clone(),
            _ => return None,
        };
        let error_rows = browse_error
            .as_deref()
            .or(self.error.as_deref())
            .map_or(0, |text| {
                usize::from(status_row_count(shown_modal_sentence(text), width))
            });
        let selected_id = match &self.drafts {
            DraftsState::Browse { selected_id, .. } => selected_id.clone(),
            _ => None,
        };
        let visible = self.visible_drafts();
        let selected = selected_id
            .as_ref()
            .and_then(|id| visible.iter().position(|draft| &draft.id == id))
            .unwrap_or(0);
        let heights = visible
            .iter()
            .map(|draft| draft_row_height(draft, width))
            .collect();
        let list_rows = usize::from(content_height).saturating_sub(2 + error_rows);
        Some((heights, selected, list_rows))
    }

    fn render_drafts(
        &mut self,
        buf: &mut Buffer,
        area: Rect,
        theme: &Theme,
        compact: bool,
    ) -> Option<FeedbackModalRender> {
        let is_searching = matches!(
            self.drafts,
            DraftsState::Browse {
                search_focused: true,
                ..
            }
        );
        let mut shortcuts = vec![
            Shortcut {
                label: "↑↓ move",
                clickable: false,
                id: 0,
            },
            Shortcut {
                label: "/ search",
                clickable: false,
                id: 0,
            },
            Shortcut {
                label: "Enter open",
                clickable: false,
                id: 0,
            },
            Shortcut {
                label: "d delete",
                clickable: false,
                id: 0,
            },
        ];
        modal_window::push_vim_nav_search_hint(&mut shortcuts, is_searching);
        let config = Self::window_config(&shortcuts, compact, false);
        let areas = modal_window::render_modal_window(buf, area, &mut self.window, &config, theme)?;
        let content = areas.content;
        self.draft_search_area = None;
        self.draft_row_areas.clear();
        let normal = ratatui::style::Style::default().fg(theme.text_secondary);
        let dim = ratatui::style::Style::default().fg(theme.gray);
        if self.discard_confirm.is_some() {
            paint_confirm(
                buf,
                content,
                "Discard the current Write composition and open the selected draft?",
                "y discard  |  n cancel",
                normal,
                dim,
            );
            return Some(FeedbackModalRender {
                cursor: None,
                post_flush: None,
            });
        }
        if let Some(draft_id) = self.delete_confirm.as_ref() {
            let prompt = if self.metadata.draft_id.as_ref() == Some(draft_id) {
                "Delete the stored recovery copy? Your current edits will remain."
            } else {
                "Delete this feedback draft?"
            };
            let answer = if self.draft_delete.is_some() {
                "Deleting…"
            } else {
                "y delete  |  n cancel"
            };
            paint_confirm(buf, content, prompt, answer, normal, dim);
            return Some(FeedbackModalRender {
                cursor: None,
                post_flush: None,
            });
        }
        if let Some((heights, selected, list_rows)) =
            self.measure_draft_list(content.width, content.height)
        {
            self.drafts_viewport_start =
                fit_draft_viewport(&heights, selected, self.drafts_viewport_start, list_rows);
        }
        match &self.drafts {
            DraftsState::Unloaded => {
                buf.set_stringn(
                    content.x,
                    content.y,
                    "Open Drafts to load saved feedback.",
                    content.width as usize,
                    dim,
                );
            }
            DraftsState::Loading { .. } => {
                buf.set_stringn(
                    content.x,
                    content.y,
                    "Loading drafts…",
                    content.width as usize,
                    dim,
                );
            }
            DraftsState::Browse {
                rows,
                selected_id,
                query,
                search_focused,
                error,
            } => {
                let mut y = content.y;
                if content.height > 0 {
                    self.draft_search_area = Some(Rect::new(content.x, y, content.width, 1));
                    crate::views::picker::render_line_editor_search_bar(
                        buf,
                        content.x,
                        y,
                        content.width,
                        theme,
                        query,
                        *search_focused,
                        true,
                        Some(theme.bg_base),
                    );
                    y = y.saturating_add(1);
                }
                if y < content.bottom() {
                    crate::views::picker::render_divider(
                        buf,
                        content.x,
                        y,
                        content.width,
                        theme,
                        Some(theme.bg_base),
                    );
                    y = y.saturating_add(1);
                }
                if let Some(error) = error.as_deref().or(self.error.as_deref())
                    && y < content.bottom()
                {
                    let error = shown_modal_sentence(error);
                    let painted = paint_wrapped_status(
                        buf,
                        Rect {
                            x: content.x,
                            y,
                            width: content.width,
                            height: content.bottom().saturating_sub(y),
                        },
                        error,
                        ratatui::style::Style::default().fg(theme.accent_error),
                    );
                    y = y.saturating_add(painted);
                }
                let visible: Vec<_> = self.visible_drafts().into_iter().cloned().collect();
                let mut rendered = 0usize;
                for draft in visible.into_iter().skip(self.drafts_viewport_start) {
                    if y >= content.bottom() || content.width == 0 {
                        break;
                    }
                    let is_selected = selected_id.as_ref() == Some(&draft.id);
                    // A model draft stores its title separately from the
                    // structured details, which start at "What happened:".
                    // The title is last on the row, so wrap instead of clipping it.
                    let line = draft_row_line(&draft, is_selected, theme);
                    let row_top = y;
                    let mut painted = 0u16;
                    for wrapped_line in
                        crate::render::wrapping::word_wrap_line(&line, usize::from(content.width))
                    {
                        if y >= content.bottom() {
                            break;
                        }
                        buf.set_line(content.x, y, &wrapped_line, content.width);
                        if is_selected {
                            // Modal-list band (`bg_highlight`, reverse when bandless) after the
                            // text, so padding cells get it. Spans omit a background, so the
                            // title stays bold primary and the classification stays secondary.
                            buf.set_style(
                                Rect::new(content.x, y, content.width, 1),
                                crate::views::modal_list::highlight(theme),
                            );
                        }
                        y += 1;
                        painted += 1;
                    }
                    if painted == 0 {
                        break;
                    }
                    self.draft_row_areas.push((
                        draft.id.clone(),
                        Rect::new(content.x, row_top, content.width, painted),
                    ));
                    rendered += 1;
                    if y >= content.bottom() {
                        break;
                    }
                }
                if rendered == 0 && y < content.bottom() {
                    let empty = if rows.is_empty() {
                        "No drafts."
                    } else {
                        "No matching drafts."
                    };
                    buf.set_stringn(content.x, y, empty, content.width as usize, dim);
                }
            }
        }
        Some(FeedbackModalRender {
            cursor: None,
            post_flush: None,
        })
    }

    /// Rows past the content height are not drawn; the disclosure wraps before the choices.
    fn render_trace_step(
        &self,
        buf: &mut Buffer,
        content: Rect,
        theme: &Theme,
        selected: FeedbackTraceChoice,
    ) {
        if content.width == 0 {
            return;
        }
        let width = content.width as usize;
        let bottom = content.y.saturating_add(content.height);
        let mut y = content.y;
        let line = |buf: &mut Buffer, y: &mut u16, text: &str, style: ratatui::style::Style| {
            if *y < bottom {
                buf.set_stringn(content.x, *y, text, width, style);
                *y += 1;
            }
        };
        line(
            buf,
            &mut y,
            self.trace_prompt(),
            ratatui::style::Style::default().fg(theme.text_primary),
        );
        let disclosure = ratatui::text::Line::styled(
            "One archive of this session is sent with this report only. Nothing is turned on for future sessions.",
            ratatui::style::Style::default().fg(theme.gray),
        );
        for wrapped in crate::render::wrapping::word_wrap_line(&disclosure, width) {
            if y >= bottom {
                break;
            }
            buf.set_line(content.x, y, &wrapped, content.width);
            y += 1;
        }
        line(buf, &mut y, "", ratatui::style::Style::default());
        for (index, choice) in FeedbackTraceChoice::ALL.into_iter().enumerate() {
            let (marker, style) = if choice == selected {
                (
                    "\u{276F}",
                    ratatui::style::Style::default()
                        .fg(theme.text_primary)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
            } else {
                (" ", ratatui::style::Style::default().fg(theme.gray))
            };
            line(
                buf,
                &mut y,
                &format!("{marker} {}. {}", index + 1, choice.label()),
                style,
            );
        }
        if let Some(error) = &self.error {
            let error = shown_modal_sentence(error);
            line(buf, &mut y, "", ratatui::style::Style::default());
            paint_wrapped_status(
                buf,
                Rect {
                    x: content.x,
                    y,
                    width: content.width,
                    height: bottom.saturating_sub(y),
                },
                error,
                ratatui::style::Style::default().fg(theme.accent_error),
            );
        }
    }

    pub(super) fn prompt_style() -> PromptStyle {
        PromptStyle {
            show_prefix: false,
            placeholder_when_focused: true,
            placeholder_override: Some("Tell us what happened"),
            // Chips only: the fullscreen preview overlay would paint over the modal.
            image_preview: false,
            ..PromptStyle::overlay()
        }
    }

    const WRITE_SHORTCUTS: &[Shortcut<'static>] = &[
        Shortcut {
            label: "Enter submit",
            clickable: false,
            id: 0,
        },
        Shortcut {
            label: "Esc cancel",
            clickable: true,
            id: CANCEL_SHORTCUT_ID,
        },
    ];

    const UNKNOWN_SUBMIT_SHORTCUTS: &[Shortcut<'static>] = &[Shortcut {
        label: "Esc close",
        clickable: false,
        id: 0,
    }];

    const TERMINAL_SUBMIT_SHORTCUTS: &[Shortcut<'static>] = &[Shortcut {
        label: "Esc close",
        clickable: false,
        id: 0,
    }];

    /// Same as [`Self::WRITE_SHORTCUTS`] plus the label-row hint; shown only when enums were supplied.
    const WRITE_SHORTCUTS_WITH_LABELS: &[Shortcut<'static>] = &[
        Shortcut {
            label: "Enter submit",
            clickable: false,
            id: 0,
        },
        Shortcut {
            label: "↑/Tab labels",
            clickable: false,
            id: 0,
        },
        Shortcut {
            label: "Esc cancel",
            clickable: true,
            id: CANCEL_SHORTCUT_ID,
        },
    ];

    // Esc while a label row is focused returns to the composer, so no cancel hint here.
    const METADATA_SHORTCUTS: &[Shortcut<'static>] = &[
        Shortcut {
            label: "↑↓ move",
            clickable: false,
            id: 0,
        },
        Shortcut {
            label: "←→ value",
            clickable: false,
            id: 0,
        },
        Shortcut {
            label: "Enter edit",
            clickable: false,
            id: 0,
        },
        Shortcut {
            label: "Tab done",
            clickable: false,
            id: 0,
        },
    ];

    // Not clickable: a click must not confirm a consent choice, and Esc here backs out instead of cancelling.
    const TRACE_SHORTCUTS: &[Shortcut<'static>] = &[
        Shortcut {
            label: "Enter select",
            clickable: false,
            id: 0,
        },
        Shortcut {
            label: "Esc back",
            clickable: false,
            id: 2,
        },
    ];

    pub(super) fn window_config<'a>(
        shortcuts: &'a [Shortcut<'a>],
        compact: bool,
        trace_step: bool,
    ) -> ModalWindowConfig<'a> {
        let mut sizing = ModalSizing::large().with_compact(compact);
        if trace_step {
            sizing.v_margin = sizing.v_margin.min(3);
        }
        ModalWindowConfig {
            title: "Feedback",
            tabs: Some(FEEDBACK_TABS),
            shortcuts,
            sizing,
            fold_info: None,
        }
    }
}
