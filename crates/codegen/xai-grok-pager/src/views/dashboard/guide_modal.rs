//! Dashboard host for `/release-notes`, `/docs`, and `/help`.
//!
//! The agent view stores these on `AgentView::active_modal`. The dashboard has no agent, so
//! [`DashboardState::guide_modal`] owns the same `ActiveModal` variants and their input.

use crossterm::event::{Event, KeyEventKind};

use crate::app::app_view::InputOutcome;
use crate::docs::DocEntry;
use crate::views::modal::ActiveModal;
use crate::views::modal_window::{self as mw, ModalWindowConfig, ModalWindowOutcome};
use crate::views::picker::{PickerConfig, PickerOutcome, handle_picker_input};

use super::state::DashboardState;

impl DashboardState {
    pub(super) fn handle_guide_modal_input(&mut self, ev: &Event) -> InputOutcome {
        if let Event::Key(key) = ev
            && key.kind == KeyEventKind::Release
        {
            return InputOutcome::Unchanged;
        }
        if matches!(self.guide_modal, Some(ActiveModal::DocViewer { .. })) {
            return self.handle_guide_doc_viewer(ev);
        }
        if matches!(self.guide_modal, Some(ActiveModal::DocPicker { .. })) {
            return self.handle_guide_doc_picker(ev);
        }
        if matches!(self.guide_modal, Some(ActiveModal::CommandPalette { .. })) {
            return self.handle_guide_command_palette(ev);
        }
        InputOutcome::Unchanged
    }

    fn handle_guide_doc_viewer(&mut self, ev: &Event) -> InputOutcome {
        let chrome = empty_chrome();
        let step = match ev {
            Event::Key(key) => {
                let ActiveModal::DocViewer { window, scroll, .. } =
                    self.guide_modal.as_mut().expect("doc viewer")
                else {
                    return InputOutcome::Unchanged;
                };
                match mw::handle_modal_key(window, key, &chrome) {
                    ModalWindowOutcome::CloseRequested => GuideStep::Close,
                    ModalWindowOutcome::Unhandled => {
                        if crate::views::modal::apply_doc_scroll(key.code, scroll) {
                            GuideStep::Changed
                        } else {
                            GuideStep::Unchanged
                        }
                    }
                    _ => GuideStep::Changed,
                }
            }
            Event::Mouse(mouse) => {
                let ActiveModal::DocViewer { window, scroll, .. } =
                    self.guide_modal.as_mut().expect("doc viewer")
                else {
                    return InputOutcome::Unchanged;
                };
                match mw::handle_modal_mouse(window, mouse.kind, mouse.column, mouse.row) {
                    ModalWindowOutcome::CloseRequested => GuideStep::Close,
                    ModalWindowOutcome::Unhandled => {
                        if crate::views::modal::apply_doc_mouse_scroll(mouse.kind, scroll) {
                            GuideStep::Changed
                        } else {
                            GuideStep::Unchanged
                        }
                    }
                    _ => GuideStep::Changed,
                }
            }
            _ => GuideStep::Unchanged,
        };
        match step {
            GuideStep::Close => {
                self.close_guide_doc_viewer();
                InputOutcome::Changed
            }
            GuideStep::Changed => InputOutcome::Changed,
            GuideStep::Unchanged | GuideStep::OpenHowto => InputOutcome::Unchanged,
        }
    }

    fn close_guide_doc_viewer(&mut self) {
        let standalone = matches!(
            self.guide_modal,
            Some(ActiveModal::DocViewer {
                standalone: true,
                ..
            })
        );
        if standalone {
            self.guide_modal = None;
            return;
        }
        let previous = match self.guide_modal.as_mut() {
            Some(ActiveModal::DocViewer {
                previous_palette, ..
            }) => previous_palette.take(),
            _ => None,
        };
        self.guide_modal = Some(crate::views::modal::howto_list_modal(previous));
    }

    fn handle_guide_doc_picker(&mut self, ev: &Event) -> InputOutcome {
        let (outcome, indices) = {
            let Some(ActiveModal::DocPicker { entries, state, .. }) = self.guide_modal.as_mut()
            else {
                return InputOutcome::Unchanged;
            };
            let indices = matching_doc_indices(entries, state.query());
            let non_sel = vec![false; indices.len()];
            let config = doc_picker_config(&non_sel);
            let outcome = handle_picker_input(ev, state, indices.len(), &config);
            (outcome, indices)
        };
        match outcome {
            PickerOutcome::Selected(i) => {
                let Some(orig) = indices.get(i).copied() else {
                    return InputOutcome::Changed;
                };
                let opened = {
                    let Some(ActiveModal::DocPicker {
                        entries,
                        previous_palette,
                        ..
                    }) = self.guide_modal.as_mut()
                    else {
                        return InputOutcome::Changed;
                    };
                    let Some(doc) = entries.get(orig) else {
                        return InputOutcome::Changed;
                    };
                    Some((
                        doc.title.clone(),
                        doc.content.to_string(),
                        previous_palette.take(),
                    ))
                };
                if let Some((title, content, previous)) = opened {
                    self.guide_modal = Some(ActiveModal::DocViewer {
                        title,
                        content,
                        scroll: 0,
                        window: mw::ModalWindowState::new(),
                        cached_lines: None,
                        previous_palette: previous,
                        standalone: false,
                    });
                }
                InputOutcome::Changed
            }
            PickerOutcome::Closed => {
                let previous = match self.guide_modal.as_mut() {
                    Some(ActiveModal::DocPicker {
                        previous_palette, ..
                    }) => previous_palette.take(),
                    _ => None,
                };
                if let Some(snapshot) = previous {
                    self.guide_modal = Some(ActiveModal::CommandPalette {
                        entries: snapshot.entries,
                        state: snapshot.state,
                        window: mw::ModalWindowState::new(),
                    });
                } else {
                    self.guide_modal = None;
                }
                InputOutcome::Changed
            }
            PickerOutcome::Unchanged => InputOutcome::Unchanged,
            _ => InputOutcome::Changed,
        }
    }

    fn handle_guide_command_palette(&mut self, ev: &Event) -> InputOutcome {
        use crate::views::modal::PaletteCommand;
        let step = {
            let Some(ActiveModal::CommandPalette { entries, state, .. }) =
                self.guide_modal.as_mut()
            else {
                return InputOutcome::Unchanged;
            };
            let filtered = crate::views::modal::filter_palette_list(entries, state.query());
            let non_sel: Vec<bool> = filtered
                .iter()
                .map(|entry| matches!(entry.command, PaletteCommand::SectionHeader(_)))
                .collect();
            let config = palette_picker_config(&non_sel);
            match handle_picker_input(ev, state, filtered.len(), &config) {
                PickerOutcome::Selected(i) => {
                    if matches!(
                        filtered.get(i).map(|entry| &entry.command),
                        Some(PaletteCommand::HowTo)
                    ) {
                        GuideStep::OpenHowto
                    } else {
                        GuideStep::Changed
                    }
                }
                PickerOutcome::Closed => GuideStep::Close,
                PickerOutcome::Unchanged => GuideStep::Unchanged,
                _ => GuideStep::Changed,
            }
        };
        match step {
            GuideStep::OpenHowto => {
                let snapshot = match self.guide_modal.as_ref() {
                    Some(ActiveModal::CommandPalette { entries, state, .. }) => {
                        Some(crate::views::modal::PaletteSnapshot {
                            entries: entries.clone(),
                            state: state.clone(),
                        })
                    }
                    _ => None,
                };
                self.guide_modal = Some(crate::views::modal::howto_list_modal(snapshot));
                InputOutcome::Changed
            }
            GuideStep::Close => {
                self.guide_modal = None;
                InputOutcome::Changed
            }
            GuideStep::Changed => InputOutcome::Changed,
            GuideStep::Unchanged => InputOutcome::Unchanged,
        }
    }
}

enum GuideStep {
    Close,
    Changed,
    Unchanged,
    OpenHowto,
}

fn matching_doc_indices(entries: &[DocEntry], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..entries.len()).collect();
    }
    let query = query.to_lowercase();
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            entry.title.to_lowercase().contains(&query)
                || entry.description.to_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect()
}

fn empty_chrome() -> ModalWindowConfig<'static> {
    ModalWindowConfig {
        title: "",
        tabs: None,
        shortcuts: &[],
        sizing: mw::ModalSizing::default(),
        fold_info: None,
    }
}

fn doc_picker_config(non_sel: &[bool]) -> PickerConfig<'_> {
    PickerConfig {
        title: Some("How-to guides"),
        show_search_hint: false,
        expandable: false,
        esc_clears_query: true,
        shortcuts: Some(crate::views::picker::picker_shortcuts()),
        pending_hint: None,
        non_selectable: non_sel,
        non_selectable_clickable: &[],
        shortcuts_area: None,
        tabs: None,
        active_tab: 0,
        filter_label: None,
        filter_key_hint: None,
        filter_active: false,
        header_note: None,
        action_keys: &[],
        disable_search: false,
        compact_bottom_bar: false,
        search_only_on_slash: false,
        vim_normal_first: crate::appearance::cache::load_vim_mode(),
        session_fold_keys: false,
    }
}

fn palette_picker_config(non_sel: &[bool]) -> PickerConfig<'_> {
    PickerConfig {
        title: None,
        show_search_hint: false,
        expandable: false,
        esc_clears_query: true,
        shortcuts: Some(crate::views::picker::picker_shortcuts()),
        pending_hint: None,
        non_selectable: non_sel,
        non_selectable_clickable: &[],
        shortcuts_area: None,
        tabs: None,
        active_tab: 0,
        filter_label: None,
        filter_key_hint: None,
        filter_active: false,
        header_note: None,
        action_keys: &[],
        disable_search: false,
        compact_bottom_bar: false,
        search_only_on_slash: false,
        vim_normal_first: crate::appearance::cache::load_vim_mode(),
        session_fold_keys: false,
    }
}
