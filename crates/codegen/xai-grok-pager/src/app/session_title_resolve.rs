//! Resume-by-title selection shared by startup paths.
//!
//! The composition root pins an explicit non-id resume target to its canonical session id BEFORE the irreversible OS sandbox
//! ([`super::cli::PagerArgs::pin_local_resume_target`]).
//! That way the saved-profile peek and materialization act on one immutable target instead of racing a concurrent rename between two title lookups.
//! Materialization keeps [`select_by_title`] as the authoritative error source (ambiguity / no-match).
//! It is also the fallback for callers that bypass pinning.

use xai_grok_shell::session::persistence::{RecentSessionSelection, Summary};

/// UUID-shaped resume args always take the id path, even when no such id exists and a session is titled with that exact UUID.
pub(crate) fn is_uuid_shaped(arg: &str) -> bool {
    uuid::Uuid::try_parse(arg).is_ok()
}

/// Canonical key for title equality.
/// Case-insensitive, and every run of non-alphanumeric characters is one space, so
/// `fix-login-bug` matches "Fix login bug". Not full Unicode caseless matching.
fn title_key(s: &str) -> String {
    let mut out = String::new();
    let mut spaced = false;
    for ch in s.trim().chars() {
        if ch.is_alphanumeric() {
            spaced = false;
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        } else if !spaced && !out.is_empty() {
            spaced = true;
            out.push(' ');
        }
    }
    out
}

/// Hint appended to every terminal failure for a non-id resume target.
/// The title miss stays visible even when remote restore produces the final error.
/// Debug formatting: the arg is arbitrary user text.
pub(crate) fn title_miss_hint(arg: &str) -> String {
    format!(
        "no session id or title matched {arg:?} for this directory; \
         try `grok sessions search {arg:?}`"
    )
}

/// `Ok(Some)`: exactly one match, or a sole manual `/rename` among duplicates (explicit user intent beats colliding auto titles).
/// `Err`: ambiguous; never silently pick one, headless scripts need determinism.
/// Candidate titles are Debug-escaped: `/rename` accepts arbitrary text, and raw control characters would corrupt the listing.
pub(crate) fn select_by_title<'a>(
    arg: &str,
    summaries: &'a [Summary],
) -> anyhow::Result<Option<&'a Summary>> {
    select_by_title_in(arg, summaries, None)
}

/// Same as [`select_by_title`]. `current_cwd` only orders an ambiguous listing: the current folder, then its checkouts, newest first inside each band.
pub(crate) fn select_by_title_in<'a>(
    arg: &str,
    summaries: &'a [Summary],
    current_cwd: Option<&str>,
) -> anyhow::Result<Option<&'a Summary>> {
    use xai_grok_shell::session::resume_match::{
        folder_band, query_matches, separator_insensitive_title,
    };
    if is_uuid_shaped(arg) {
        return Ok(None);
    }
    if title_key(arg).is_empty() {
        return Ok(None);
    }
    let mut exact: Vec<&Summary> = Vec::new();
    let mut fuzzy: Vec<&Summary> = Vec::new();
    for summary in summaries {
        if separator_insensitive_title(summary.display_title(), arg) {
            exact.push(summary);
        } else if query_matches(
            "",
            &[summary.display_title(), summary.session_summary.as_str()],
            arg,
        ) {
            fuzzy.push(summary);
        }
    }
    let tier = if !exact.is_empty() { exact } else { fuzzy };
    match tier.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some(*only)),
        _ => {
            let manual: Vec<&&Summary> = tier
                .iter()
                .filter(|summary| {
                    summary
                        .manual_title_opt()
                        .is_some_and(|title| separator_insensitive_title(&title, arg))
                })
                .collect();
            if let [only] = manual.as_slice() {
                return Ok(Some(**only));
            }
            let mut ordered = tier.clone();
            ordered.sort_by(|left, right| {
                folder_band(
                    &left.info.cwd,
                    left.git_root_dir.as_deref(),
                    left.source_workspace_dir.as_deref(),
                    left.worktree_label.as_deref(),
                    current_cwd,
                )
                .cmp(&folder_band(
                    &right.info.cwd,
                    right.git_root_dir.as_deref(),
                    right.source_workspace_dir.as_deref(),
                    right.worktree_label.as_deref(),
                    current_cwd,
                ))
                .then(right.updated_at.cmp(&left.updated_at))
                .then(left.info.id.to_string().cmp(&right.info.id.to_string()))
            });
            let listing = ordered
                .iter()
                .map(|summary| {
                    format!(
                        "  {}  {:?}  updated {}",
                        summary.info.id,
                        summary.display_title(),
                        summary.updated_at.to_rfc3339()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::bail!(
                "Multiple sessions match title {:?}:\n{listing}\n\
                 Resume by session id instead: grok --resume <session-id>",
                arg.trim()
            );
        }
    }
}

/// Miss text plus up to five near titles, newest-first as the caller ordered `titles`.
pub(crate) fn title_miss_with_similar(arg: &str, titles: &[String]) -> Option<String> {
    let similar = xai_grok_shell::session::resume_match::similar_titles(arg, titles);
    if similar.is_empty() {
        return None;
    }
    let mut message = title_miss_hint(arg);
    message.push_str("\nSimilar titles:");
    for title in similar {
        message.push_str("\n  ");
        message.push_str(&format!("{title:?}"));
    }
    Some(message)
}

/// Outcome of the pre-sandbox resolution of an explicit resume arg.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PinnedResumeTarget {
    /// Nothing local resolved (UUID-shaped, no cwd, junk, or ambiguous title): leave the raw arg alone.
    /// Materialization owns the authoritative error / remote path.
    Unresolved,
    /// Resolved as a local id (possibly the restored child of a remote id).
    Id(String),
    /// Resolved by title to this session. The selected summary's persisted sandbox profile rides along.
    /// Re-deriving the profile from the id is ambiguous when a legacy id is duplicated across cwd dirs.
    Title {
        id: String,
        sandbox_profile: Option<String>,
    },
}

impl PinnedResumeTarget {
    pub(crate) fn id(self) -> Option<String> {
        match self {
            Self::Unresolved => None,
            Self::Id(id) | Self::Title { id, .. } => Some(id),
        }
    }
}

/// The saved-profile peek and materialization must consume one immutable target, not re-run title selection against mutable summaries.
/// That preserves the restored-child id so the peek cannot drift to a same-id session in another cwd.
/// Errs on a listing failure (fail closed instead of guessing) and on ambiguity, which must be reported before the sandbox rather than after it.
pub(crate) fn presandbox_resume_target(
    arg: &str,
    cwd: Option<&str>,
    selection: RecentSessionSelection,
) -> anyhow::Result<PinnedResumeTarget> {
    if is_uuid_shaped(arg) {
        return Ok(PinnedResumeTarget::Unresolved);
    }
    let Some(cwd) = cwd else {
        return Ok(PinnedResumeTarget::Unresolved);
    };
    if let Some(local_id) = xai_grok_shell::session::resolve_local_session(arg, cwd) {
        return Ok(PinnedResumeTarget::Id(local_id));
    }
    if xai_grok_shell::session::resolve_local_session_any_cwd(arg).is_some() {
        return Ok(PinnedResumeTarget::Id(arg.to_string()));
    }
    let summaries = xai_grok_shell::session::persistence::local_summaries_all_sync(selection)
        .map_err(|e| {
            anyhow::anyhow!("failed to list local sessions while resolving --resume {arg:?}: {e}")
        })?;
    match select_by_title_in(arg, &summaries, Some(cwd))? {
        Some(summary) => Ok(PinnedResumeTarget::Title {
            id: summary.info.id.to_string(),
            sandbox_profile: summary.sandbox_profile.clone(),
        }),
        None => {
            let titles = summaries
                .iter()
                .map(|summary| summary.display_title().to_string())
                .collect::<Vec<_>>();
            if let Some(message) = title_miss_with_similar(arg, &titles) {
                anyhow::bail!(message);
            }
            Ok(PinnedResumeTarget::Unresolved)
        }
    }
}

/// `local_miss_target` is `Some(arg)` only when materialization deferred exactly this target after missing local id/title resolution.
/// Provenance is threaded, never inferred from id shape, so a resolved legacy non-UUID id gets no false no-match hint.
/// `detail` must already be user-sanitized: sanitizing the composed message would collapse disk-full chains whole and erase the appended hint.
pub(crate) fn worktree_resume_failure_message(
    local_miss_target: Option<&str>,
    detail: &str,
) -> String {
    let msg = format!("couldn't resume worktree session: {detail}");
    match local_miss_target {
        Some(target) => format!("{msg}; {}", title_miss_hint(target)),
        None => msg,
    }
}

#[cfg(test)]
#[path = "session_title_resolve_tests.rs"]
mod tests;
