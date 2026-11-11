//! Action commands for the HunkTrackerActor.
//!
//! These methods handle accept/reject actions on hunks.
//!
//! # Invariants
//!
//! Hunks only exist for files where both baseline and current content are
//! patchable (i.e., `FileContentState::Full` or `FileContentState::Missing`
//! for creation/deletion). Files with `Binary` or `TooLarge` content states
//! have their hunks cleared by `recompute_hunks()`.
//!
//! This means action handlers should never encounter a non-patchable state
//! when processing a hunk. The guards in these methods are defensive fallbacks
//! that silently skip non-patchable states rather than panic.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::diff::{patch_lines, revert_rejected_hunks};
use crate::events::{HunkEvent, HunkRemovalReason};
use crate::types::{Hunk, HunkAction, HunkActionError, HunkId, HunkLineInfo, HunkSource};

use super::HunkTrackerActor;
use super::state::FileContentState;

impl HunkTrackerActor {
    /// Update session stats when a hunk is accepted or rejected.
    fn update_session_stats(&mut self, line_info: &HunkLineInfo, accepted: bool) {
        let lines_added = line_info.new_count;
        let lines_removed = line_info.old_count;

        if accepted {
            self.session_stats.accepted_hunks = self.session_stats.accepted_hunks.saturating_add(1);
            self.session_stats.accepted_lines_added = self
                .session_stats
                .accepted_lines_added
                .saturating_add(lines_added);
            self.session_stats.accepted_lines_removed = self
                .session_stats
                .accepted_lines_removed
                .saturating_add(lines_removed);
        } else {
            self.session_stats.rejected_hunks = self.session_stats.rejected_hunks.saturating_add(1);
            self.session_stats.rejected_lines_added = self
                .session_stats
                .rejected_lines_added
                .saturating_add(lines_added);
            self.session_stats.rejected_lines_removed = self
                .session_stats
                .rejected_lines_removed
                .saturating_add(lines_removed);
        }
    }

    /// Remove a hunk from turn_index based on its source.
    fn remove_from_turn_index(&mut self, hunk_id: &HunkId, source: &HunkSource) {
        if let Some(prompt_index) = source.prompt_index()
            && let Some(set) = self.turn_index.get_mut(&prompt_index)
        {
            set.remove(hunk_id);
        }
    }

    /// Apply action (accept/reject) to a specific hunk.
    pub(super) async fn apply_hunk_action(
        &mut self,
        hunk_id: &HunkId,
        action: HunkAction,
    ) -> Result<(), HunkActionError> {
        // Find which file contains this hunk and capture the full hunk data
        let hunk_info = self.file_states.iter().find_map(|(path, state)| {
            state
                .hunks
                .iter()
                .find(|h| &h.id == hunk_id)
                .map(|h| (path.clone(), h.clone()))
        });

        let Some((path, hunk)) = hunk_info else {
            return Err(HunkActionError::HunkNotFound(hunk_id.clone()));
        };

        // Update session stats before removing the hunk
        let accepted = matches!(action, HunkAction::Accept);
        self.update_session_stats(&hunk.line_info, accepted);

        // Remove from turn_index
        self.remove_from_turn_index(hunk_id, &hunk.source);

        match action {
            HunkAction::Accept => {
                self.accept_hunk(&path, &hunk).await?;
            }
            HunkAction::Reject => {
                self.reject_hunk(&path, &hunk).await?;
            }
        }

        Ok(())
    }

    /// Accept a single hunk: patch baseline to include this hunk's changes.
    async fn accept_hunk(&mut self, path: &Path, hunk: &Arc<Hunk>) -> Result<(), HunkActionError> {
        // Collect data and perform mutations, then send events afterward
        let (should_recompute, current, _source) = {
            let state = self.file_states.get_mut(path);
            let Some(state) = state else {
                return Ok(());
            };

            // Handle file creation case (no baseline)
            if matches!(state.baseline, FileContentState::Missing) {
                // File was created - accepting makes current the baseline for this hunk's lines
                // For a newly created file, accept just sets baseline = current
                state.baseline = state.current_content.clone();
                state.baseline_accepted = true;
                state.hunks.retain(|h| h.id != hunk.id);
                // Don't recompute for file creation (no other hunks exist)
                (false, None, hunk.source)
            } else {
                // Patch baseline to include ONLY this hunk's changes
                if let FileContentState::Full(baseline) = &state.baseline {
                    let patched = patch_lines(
                        baseline,
                        hunk.line_info.old_start,
                        hunk.line_info.old_count,
                        &hunk.new_text,
                    );
                    state.baseline = FileContentState::Full(patched);
                }
                state.baseline_accepted = true;

                // Remove this hunk from the list
                state.hunks.retain(|h| h.id != hunk.id);

                // Pass FileContentState directly (R3/MF-4: preserve Binary/TooLarge, no String extraction)
                let current = Some(state.current_content.clone());
                let source = hunk.source;
                (true, current, source)
            }
        };

        // Send events after mutable borrow is released
        self.send_event(HunkEvent::HunkRemoved {
            path: path.to_path_buf(),
            hunk_id: hunk.id.clone(),
            reason: HunkRemovalReason::Accepted,
        });
        self.send_event(HunkEvent::BaselineUpdated {
            path: path.to_path_buf(),
        });

        // Recompute remaining hunks (positions may have shifted)
        if should_recompute {
            // Use the source of the first remaining hunk if any, else External
            let remaining_source = self
                .file_states
                .get(path)
                .and_then(|state| state.hunks.first())
                .map(|h| h.source)
                .unwrap_or(HunkSource::External);

            self.recompute_hunks(path, current, remaining_source);
        }

        Ok(())
    }

    /// Reject a single hunk. Shares the batch path so a missed user save is kept there too.
    async fn reject_hunk(&mut self, path: &Path, hunk: &Arc<Hunk>) -> Result<(), HunkActionError> {
        self.reject_hunks_batch(path, std::slice::from_ref(hunk))
            .await
    }

    /// Apply action (accept/reject) to all hunks for a file.
    /// Uses batched processing to avoid stale hunk IDs during recomputation.
    pub(super) async fn apply_file_action(
        &mut self,
        path: &Path,
        action: HunkAction,
    ) -> Result<Vec<HunkId>, HunkActionError> {
        let Some(state) = self.file_states.get(path) else {
            return Ok(vec![]);
        };

        let hunks_to_process: Vec<Arc<Hunk>> = state.hunks.clone();

        if hunks_to_process.is_empty() {
            return Ok(vec![]);
        }

        self.apply_action_batch(&[(path.to_path_buf(), hunks_to_process)], action)
            .await
    }

    /// Apply action (accept/reject) to all hunks.
    /// Uses batched processing to avoid stale hunk IDs during recomputation.
    pub(super) async fn apply_all_action(
        &mut self,
        action: HunkAction,
    ) -> Result<Vec<HunkId>, HunkActionError> {
        // Collect all hunks grouped by file
        let files_with_hunks: Vec<(PathBuf, Vec<Arc<Hunk>>)> = self
            .file_states
            .iter()
            .filter(|(_, state)| !state.hunks.is_empty())
            .map(|(path, state)| (path.clone(), state.hunks.clone()))
            .collect();

        if files_with_hunks.is_empty() {
            return Ok(vec![]);
        }

        self.apply_action_batch(&files_with_hunks, action).await
    }

    /// Apply action (accept/reject) to all hunks for a specific turn.
    /// Uses batched processing to avoid stale hunk IDs during recomputation.
    pub(super) async fn apply_turn_action(
        &mut self,
        prompt_index: usize,
        action: HunkAction,
    ) -> Result<Vec<HunkId>, HunkActionError> {
        // Collect all hunks for this turn, grouped by file
        let mut files_with_hunks: std::collections::HashMap<PathBuf, Vec<Arc<Hunk>>> =
            std::collections::HashMap::new();

        for (path, state) in &self.file_states {
            let turn_hunks: Vec<Arc<Hunk>> = state
                .hunks
                .iter()
                .filter(|h| h.source.prompt_index() == Some(prompt_index))
                .cloned()
                .collect();

            if !turn_hunks.is_empty() {
                files_with_hunks.insert(path.clone(), turn_hunks);
            }
        }

        if files_with_hunks.is_empty() {
            return Ok(vec![]);
        }

        let files_vec: Vec<_> = files_with_hunks.into_iter().collect();
        self.apply_action_batch(&files_vec, action).await
    }

    /// Internal batched action processor.
    /// Processes all hunks for each file atomically, then recomputes once per file.
    async fn apply_action_batch(
        &mut self,
        files_with_hunks: &[(PathBuf, Vec<Arc<Hunk>>)],
        action: HunkAction,
    ) -> Result<Vec<HunkId>, HunkActionError> {
        let mut affected_hunk_ids = Vec::new();

        for (path, hunks) in files_with_hunks {
            // Process all hunks for this file in one go
            for hunk in hunks {
                // Update session stats
                let accepted = matches!(action, HunkAction::Accept);
                self.update_session_stats(&hunk.line_info, accepted);

                // Remove from turn_index
                self.remove_from_turn_index(&hunk.id, &hunk.source);

                affected_hunk_ids.push(hunk.id.clone());
            }

            // Apply all patches for this file at once
            match action {
                HunkAction::Accept => {
                    self.accept_hunks_batch(path, hunks).await?;
                }
                HunkAction::Reject => {
                    self.reject_hunks_batch(path, hunks).await?;
                }
            }
        }

        Ok(affected_hunk_ids)
    }

    /// Accept multiple hunks for a file atomically.
    /// Patches baseline incrementally, then recomputes once.
    async fn accept_hunks_batch(
        &mut self,
        path: &Path,
        hunks: &[Arc<Hunk>],
    ) -> Result<(), HunkActionError> {
        if hunks.is_empty() {
            return Ok(());
        }

        let (should_recompute, current) = {
            let Some(state) = self.file_states.get_mut(path) else {
                return Ok(());
            };

            // Special case: file creation (no baseline)
            if matches!(state.baseline, FileContentState::Missing) {
                // Accepting all hunks for a new file just sets baseline = current
                state.baseline = state.current_content.clone();
                state.baseline_accepted = true;
                let hunk_ids: Vec<HunkId> = hunks.iter().map(|h| h.id.clone()).collect();
                state.hunks.retain(|h| !hunk_ids.contains(&h.id));

                // Send events
                for hunk in hunks {
                    self.send_event(HunkEvent::HunkRemoved {
                        path: path.to_path_buf(),
                        hunk_id: hunk.id.clone(),
                        reason: HunkRemovalReason::Accepted,
                    });
                }
                self.send_event(HunkEvent::BaselineUpdated {
                    path: path.to_path_buf(),
                });

                return Ok(());
            }

            // Patch baseline for each hunk (process end-to-start to avoid shifts)
            let mut sorted_hunks = hunks.to_vec();
            sorted_hunks.sort_by_key(|h| std::cmp::Reverse(h.line_info.old_start));
            for hunk in &sorted_hunks {
                if let FileContentState::Full(baseline) = &state.baseline {
                    let patched = patch_lines(
                        baseline,
                        hunk.line_info.old_start,
                        hunk.line_info.old_count,
                        &hunk.new_text,
                    );
                    state.baseline = FileContentState::Full(patched);
                }
            }
            state.baseline_accepted = true;

            // Remove all accepted hunks
            let hunk_ids: Vec<HunkId> = hunks.iter().map(|h| h.id.clone()).collect();
            state.hunks.retain(|h| !hunk_ids.contains(&h.id));

            // Pass FileContentState directly (R3/MF-4: preserve Binary/TooLarge)
            let current = Some(state.current_content.clone());
            (true, current)
        };

        // Send events
        for hunk in hunks {
            self.send_event(HunkEvent::HunkRemoved {
                path: path.to_path_buf(),
                hunk_id: hunk.id.clone(),
                reason: HunkRemovalReason::Accepted,
            });
        }
        self.send_event(HunkEvent::BaselineUpdated {
            path: path.to_path_buf(),
        });

        // Recompute remaining hunks once
        if should_recompute {
            // Use the source of the first remaining hunk if any, else External
            let remaining_source = self
                .file_states
                .get(path)
                .and_then(|state| state.hunks.first())
                .map(|h| h.source)
                .unwrap_or(HunkSource::External);

            self.recompute_hunks(path, current, remaining_source);
        }

        Ok(())
    }

    /// Reject multiple hunks for a file atomically.
    /// Patches the working copy, then recomputes once.
    /// A save the tracker never saw is the base of that patch, so those edits stay.
    async fn reject_hunks_batch(
        &mut self,
        path: &Path,
        hunks: &[Arc<Hunk>],
    ) -> Result<(), HunkActionError> {
        if hunks.is_empty() {
            return Ok(());
        }

        let (current_missing, baseline_missing, current_text, baseline_text) = {
            let Some(state) = self.file_states.get(path) else {
                return Ok(());
            };
            (
                matches!(state.current_content, FileContentState::Missing),
                matches!(state.baseline, FileContentState::Missing),
                match &state.current_content {
                    FileContentState::Full(text) => Some(text.clone()),
                    _ => None,
                },
                match &state.baseline {
                    FileContentState::Full(text) => Some(text.clone()),
                    _ => None,
                },
            )
        };

        let outcome = if current_missing {
            let Some(baseline) = baseline_text else {
                return Ok(());
            };
            self.reject_deleted_file(path, baseline).await?
        } else if baseline_missing {
            let Some(tracked) = current_text else {
                return Ok(());
            };
            self.reject_created_file(path, &tracked, hunks).await?
        } else {
            let Some(tracked) = current_text else {
                return Ok(());
            };
            self.reject_text_hunks(path, &tracked, hunks).await?
        };

        let Some(state) = self.file_states.get_mut(path) else {
            return Ok(());
        };
        let hunk_ids: Vec<HunkId> = hunks.iter().map(|hunk| hunk.id.clone()).collect();
        state.hunks.retain(|hunk| !hunk_ids.contains(&hunk.id));
        let recompute = match outcome {
            RejectWrite::Set { content, recompute } => {
                state.current_content = content.clone();
                recompute.then_some(content)
            }
        };

        for hunk in hunks {
            self.send_event(HunkEvent::HunkRemoved {
                path: path.to_path_buf(),
                hunk_id: hunk.id.clone(),
                reason: HunkRemovalReason::Rejected,
            });
        }

        if let Some(content) = recompute {
            let remaining_source = self
                .file_states
                .get(path)
                .and_then(|state| state.hunks.first())
                .map(|hunk| hunk.source)
                .unwrap_or(HunkSource::External);
            self.recompute_hunks(path, Some(content), remaining_source);
        }

        Ok(())
    }

    /// The file is gone in the tracker. Restore the baseline unless the user saved a copy we missed.
    async fn reject_deleted_file(
        &self,
        path: &Path,
        baseline: String,
    ) -> Result<RejectWrite, HunkActionError> {
        match super::file_utils::read_file_bounded(path).await {
            FileContentState::Missing => {
                tokio::fs::write(path, &baseline)
                    .await
                    .map_err(|source| HunkActionError::WriteError {
                        path: path.to_path_buf(),
                        source,
                    })?;
                Ok(RejectWrite::Set {
                    content: FileContentState::Full(baseline),
                    recompute: false,
                })
            }
            FileContentState::Full(text) => Ok(RejectWrite::Set {
                content: FileContentState::Full(text),
                recompute: true,
            }),
            other => Ok(RejectWrite::Set {
                content: other,
                recompute: true,
            }),
        }
    }

    /// The file did not exist at baseline. Delete it only when disk still matches the tracked create.
    async fn reject_created_file(
        &self,
        path: &Path,
        tracked: &str,
        hunks: &[Arc<Hunk>],
    ) -> Result<RejectWrite, HunkActionError> {
        match super::file_utils::read_file_bounded(path).await {
            FileContentState::Full(text) if text == tracked => {
                tokio::fs::remove_file(path)
                    .await
                    .map_err(|source| HunkActionError::DeleteError {
                        path: path.to_path_buf(),
                        source,
                    })?;
                Ok(RejectWrite::Set {
                    content: FileContentState::Missing,
                    recompute: false,
                })
            }
            FileContentState::Full(text) => {
                let patched = revert_rejected_hunks(tracked, &text, hunks);
                if patched.is_empty() {
                    match tokio::fs::remove_file(path).await {
                        Ok(()) => {}
                        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
                        Err(source) => {
                            return Err(HunkActionError::DeleteError {
                                path: path.to_path_buf(),
                                source,
                            });
                        }
                    }
                    return Ok(RejectWrite::Set {
                        content: FileContentState::Missing,
                        recompute: false,
                    });
                }
                if patched != text {
                    tokio::fs::write(path, &patched)
                        .await
                        .map_err(|source| HunkActionError::WriteError {
                            path: path.to_path_buf(),
                            source,
                        })?;
                }
                Ok(RejectWrite::Set {
                    content: FileContentState::Full(patched),
                    recompute: true,
                })
            }
            FileContentState::Missing => Ok(RejectWrite::Set {
                content: FileContentState::Missing,
                recompute: false,
            }),
            other => Ok(RejectWrite::Set {
                content: other,
                recompute: true,
            }),
        }
    }

    /// Undo the hunks in the working copy. Tracked text is used only when disk matches it.
    async fn reject_text_hunks(
        &self,
        path: &Path,
        tracked: &str,
        hunks: &[Arc<Hunk>],
    ) -> Result<RejectWrite, HunkActionError> {
        let disk_text = match super::file_utils::read_file_bounded(path).await {
            FileContentState::Full(text) => text,
            _ => tracked.to_string(),
        };
        let patched = revert_rejected_hunks(tracked, &disk_text, hunks);
        if patched != disk_text {
            tokio::fs::write(path, &patched)
                .await
                .map_err(|source| HunkActionError::WriteError {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
        Ok(RejectWrite::Set {
            content: FileContentState::Full(patched),
            recompute: true,
        })
    }
}

enum RejectWrite {
    Set {
        content: FileContentState,
        recompute: bool,
    },
}
