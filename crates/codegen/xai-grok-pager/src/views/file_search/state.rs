//! File search state: owns the fuzzy matcher daemon, results, and dropdown state.
//!
//! This is the core engine for @-completion. It manages:
//! - A background [`FuzzyFileMatcherDaemon`] that walks the directory tree
//! - The current [`AtContext`] (parsed from prompt text and cursor)
//! - Cached fuzzy match results (polled on tick)
//! - Dropdown selection state (selected index, scroll offset)
//! - Text replacement logic when a result is accepted

use std::path::{Path, PathBuf};
use std::sync::Arc;

use xai_grok_workspace::file_system::{
    FuzzyFileMatcher, FuzzyFileMatcherDaemon, FuzzyMatchResult, FuzzyMatcherDaemonResults,
};

use super::context::{self, AtContext, normalize_display_path};

/// Top-K results to request from the fuzzy matcher.
const MATCHER_TOP_K: usize = 1000;

/// Whether a new query should restart the daemon's directory walk.
enum RestartWalk {
    /// Reuse the current walk (query changed but hidden mode did not).
    Keep,
    /// Restart the walk, including or excluding hidden entries.
    Restart { hidden: bool },
}

/// Replacement to apply to the prompt text after accepting a fuzzy result.
#[derive(Debug, Clone)]
pub struct FileSearchReplacement {
    /// Byte range in the prompt text to replace (excludes the `@`).
    pub range: std::ops::Range<usize>,
    /// Replacement text (the normalized path, possibly with trailing space or `/`).
    pub text: String,
    /// Where to place the cursor after replacement.
    pub cursor: usize,
    /// Whether the @-context should be cleared (an already-present directory was committed).
    pub dismiss: bool,
}

/// Build accepted directory replacement text: append `/` for drill-down and a trailing space when the token ends the prompt.
fn accept_text(path: &str, at_end: bool) -> String {
    let mut text = path.to_owned();
    text.push('/');
    if at_end {
        text.push(' ');
    }
    text
}

/// Whether a daemon snapshot may replace the hits on screen.
/// Non-empty hits must be scored for `current_query`. An empty snapshot is applied only once matching is done.
fn accept_polled_results(
    current_query: &str,
    min_generation: usize,
    incoming: &FuzzyMatcherDaemonResults,
) -> bool {
    if incoming.generation < min_generation {
        return false;
    }
    // An untagged empty snapshot (a disabled daemon) may still clear the list. Any other query must match.
    if incoming.query != current_query && !incoming.query.is_empty() {
        return false;
    }
    !incoming.topk.is_empty() || incoming.status.done
}

/// File search state for @-completion.
pub struct FileSearchState {
    /// Directory the matcher walks.
    /// Mirrors the daemon's root (otherwise moved into its worker thread) so callers can see where `@`-completion is currently pointed.
    root: PathBuf,
    /// Background fuzzy matcher daemon, built lazily on first @-use.
    /// Eager construction spawns the nucleo pool and walker threads even in sessions that never open @-search.
    /// Deferring it moves that thread spawn, and its EAGAIN risk, to first use rather than removing it.
    daemon: Option<FuzzyFileMatcherDaemon>,
    /// Test-only count of daemon builds, to prove reuse (no drop-and-rebuild).
    #[cfg(test)]
    daemon_builds: usize,
    #[cfg(test)]
    poll_calls: u64,
    /// Latest results snapshot from the daemon.
    results: FuzzyMatcherDaemonResults,
    /// Current @-context (if cursor is inside an @-token).
    context: Option<AtContext>,
    /// Selected index in the dropdown list (keyboard-driven).
    selected: usize,
    /// Hovered index in the dropdown list (mouse-driven).
    /// `None` when the mouse is not over any item.
    hovered: Option<usize>,
    /// Scroll offset for the dropdown list.
    scroll_offset: usize,
    /// Floor for accepted result generations: the stale-result fence. Rises monotonically and is never
    /// lowered.
    min_generation: usize,
    /// Matcher query the displayed hits were scored for.
    /// `None` after the query changes, until a snapshot for the new query arrives.
    shown_query: Option<String>,
    /// Directory being drilled into; keeps the @-token alive when its name has whitespace (`my dir`).
    /// Self-validating: applies only while the path matches.
    drill_prefix: Option<String>,
    /// `@/`, `@~/`, and `@../` list that folder. `false` keeps the workspace walk.
    allow_outside_workspace: bool,
    /// Home directory `~/` lists. `None` leaves those queries empty.
    home: Option<PathBuf>,
    /// The hits on screen came from an outside-folder listing, not the workspace daemon.
    outside_listing: bool,
}

/// Outside `@` queries name a folder with `/`, `~/`, or `../`. The text through the last separator is that folder.
fn outside_dir_and_prefix(query: &str) -> Option<(&str, &str)> {
    let outside = query.starts_with('/')
        || query.starts_with("~/")
        || query.starts_with("~\\")
        || query.starts_with("../")
        || query.starts_with("..\\");
    if !outside {
        return None;
    }
    let sep = query.rfind(['/', '\\'])?;
    let dir = query.get(..=sep)?;
    let prefix = query.get(sep.saturating_add(1)..).unwrap_or("");
    Some((dir, prefix))
}

fn resolve_outside_dir(dir_text: &str, workspace: &Path, home: Option<&Path>) -> Option<PathBuf> {
    if dir_text.starts_with('/') || dir_text.starts_with('\\') {
        return Some(PathBuf::from(dir_text));
    }
    if let Some(rest) = dir_text
        .strip_prefix("~/")
        .or_else(|| dir_text.strip_prefix("~\\"))
    {
        let home = home?;
        return Some(if rest.is_empty() {
            home.to_path_buf()
        } else {
            home.join(rest)
        });
    }
    if dir_text.starts_with("../") || dir_text.starts_with("..\\") {
        return Some(workspace.join(dir_text));
    }
    None
}

/// One level of `dir`. Names that do not start with `prefix` are dropped. Dot names stay hidden unless `hidden`.
fn list_outside_folder(
    query: &str,
    workspace: &Path,
    home: Option<&Path>,
    hidden: bool,
) -> Option<Vec<FuzzyMatchResult>> {
    let (dir_text, prefix) = outside_dir_and_prefix(query)?;
    let Some(dir) = resolve_outside_dir(dir_text, workspace, home) else {
        return Some(Vec::new());
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Some(Vec::new());
    };
    let mut rows: Vec<(String, bool)> = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.is_empty() || (!hidden && name.starts_with('.')) {
            continue;
        }
        if !prefix.is_empty() && !name.starts_with(prefix) {
            continue;
        }
        let is_dir = match entry.file_type() {
            Ok(kind) if kind.is_dir() => true,
            Ok(kind) if kind.is_symlink() => entry
                .path()
                .metadata()
                .map(|meta| meta.is_dir())
                .unwrap_or(false),
            _ => false,
        };
        rows.push((name, is_dir));
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    rows.truncate(256);
    let hits = rows
        .into_iter()
        .map(|(name, is_dir)| FuzzyMatchResult {
            path: nucleo::Utf32String::from(format!("{dir_text}{name}")),
            score: 0,
            indices: Vec::new(),
            is_dir,
        })
        .collect();
    Some(hits)
}

impl FileSearchState {
    /// Create a new file search state rooted at the given path.
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_owned(),
            daemon: None,
            #[cfg(test)]
            daemon_builds: 0,
            #[cfg(test)]
            poll_calls: 0,
            results: FuzzyMatcherDaemonResults::default(),
            context: None,
            selected: 0,
            hovered: None,
            scroll_offset: 0,
            min_generation: 0,
            shown_query: None,
            drill_prefix: None,
            allow_outside_workspace: true,
            home: xai_dirs::home_dir(),
            outside_listing: false,
        }
    }

    /// Point @-completion at a new tree (e.g. after worktree creation).
    ///
    /// Drops any built daemon; the next @-use rebuilds it lazily against `root`.
    /// The outside-workspace switch and the home directory stay.
    pub fn retarget(&mut self, root: &Path) {
        let allow_outside_workspace = self.allow_outside_workspace;
        let home = self.home.clone();
        *self = Self::new(root);
        self.allow_outside_workspace = allow_outside_workspace;
        self.home = home;
    }

    /// `[ui] at_picker_outside_workspace`. `false` stops `@/`, `@~/`, and `@../` from leaving the workspace.
    pub(crate) fn set_allow_outside_workspace(&mut self, allow: bool) {
        self.allow_outside_workspace = allow;
    }

    /// Home folder used when the query starts with `~/`.
    #[cfg(test)]
    pub(crate) fn set_home_dir(&mut self, home: PathBuf) {
        self.home = Some(home);
    }

    /// The directory the matcher currently walks (the `@`-completion root).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The fuzzy matcher daemon, built lazily on first use.
    ///
    /// The first `@`-keystroke pays a one-time cost on the UI thread: building the daemon spawns the nucleo matcher pool and the walker.
    fn ensure_daemon(&mut self) -> &mut FuzzyFileMatcherDaemon {
        if self.daemon.is_none() {
            let daemon =
                FuzzyFileMatcherDaemon::new(FuzzyFileMatcher::new(&self.root), MATCHER_TOP_K);
            self.daemon = Some(daemon);
            #[cfg(test)]
            {
                self.daemon_builds += 1;
            }
        }
        self.daemon.as_mut().expect("daemon built above")
    }

    /// Point the daemon (building it if needed) at `query`, optionally restarting the directory walk, then reset dropdown selection and scroll.
    ///
    /// The matcher never filters to directories only: a trailing `/` scopes the query to a folder without hiding that folder's files.
    fn start_query(&mut self, restart: RestartWalk, query: &str) {
        let previous = self
            .context
            .as_ref()
            .map(|ctx| ctx.matcher_query().to_owned());
        let query_changed = previous.as_deref() != Some(query);
        let restarting = matches!(restart, RestartWalk::Restart { .. });
        let daemon = self.ensure_daemon();
        if let RestartWalk::Restart { hidden } = restart {
            daemon.restart_walk(hidden);
        }
        daemon.set_query(query, false);
        // Advance the stale-result fence past the prior query (see `min_generation`).
        self.min_generation += 1;
        // A fast Tab must not commit the previous query's highlighted row.
        if query_changed || restarting {
            self.results = FuzzyMatcherDaemonResults::default();
            self.shown_query = None;
        }
        self.selected = 0;
        self.hovered = None;
        self.scroll_offset = 0;
    }

    /// Whether the dropdown should be visible.
    pub fn is_visible(&self) -> bool {
        self.result_count() > 0
    }

    /// Displayed hits belong to the query currently in the `@` token.
    fn hits_are_for_current_query(&self) -> bool {
        self.context
            .as_ref()
            .is_some_and(|ctx| self.shown_query.as_deref() == Some(ctx.matcher_query()))
    }

    /// Hits are on screen but the matcher has not finished this query.
    /// Navigation and accept wait so a partial list cannot move the highlight or commit a row.
    pub fn is_scoring(&self) -> bool {
        self.hits_are_for_current_query() && !self.results.status.done
    }

    /// The current @-context, if any.
    pub fn context(&self) -> Option<&AtContext> {
        self.context.as_ref()
    }

    /// The current results snapshot.
    pub fn results(&self) -> &FuzzyMatcherDaemonResults {
        &self.results
    }

    /// Currently selected index in the results.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Scroll offset for the dropdown.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Currently hovered index (mouse-driven), if any.
    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    /// Set the hovered index. Returns `true` if changed.
    pub fn set_hovered(&mut self, index: Option<usize>) -> bool {
        if self.is_scoring() {
            return false;
        }
        let clamped = index.filter(|&i| i < self.results.topk.len());
        let changed = clamped != self.hovered;
        self.hovered = clamped;
        changed
    }

    /// Whether the current query is in directory-only mode.
    pub fn is_dir_mode(&self) -> bool {
        self.context.as_ref().is_some_and(|c| c.is_dir_mode())
    }

    /// Anchor (or clear) the drilled directory for whitespace-aware detection.
    pub fn set_drill_prefix(&mut self, prefix: Option<String>) {
        self.drill_prefix = prefix;
    }

    /// Recompute the @-context from the current prompt text and cursor position.
    ///
    /// Called after every text change or cursor movement.
    pub fn update_context(&mut self, text: &str, cursor: usize) {
        let new_ctx = context::detect_with_drill(text, cursor, self.drill_prefix.as_deref());

        let anchor_stale = new_ctx.as_ref().is_some_and(|new| {
            self.drill_prefix.as_deref().is_some_and(|prefix| {
                !text
                    .get(new.path_range().start..)
                    .is_some_and(|rest| rest.starts_with(prefix))
            })
        });
        if anchor_stale {
            self.drill_prefix = None;
        }

        if self.allow_outside_workspace
            && let Some(ctx) = new_ctx.clone()
            && let Some(hits) = list_outside_folder(
                ctx.matcher_query(),
                &self.root,
                self.home.as_deref(),
                ctx.is_hidden_mode(),
            )
        {
            if self.context.is_none() {
                self.drill_prefix = None;
            }
            self.install_outside_listing(ctx, hits);
            return;
        }

        self.outside_listing = false;
        match (&self.context, &new_ctx) {
            (None, Some(ctx)) => {
                // Fresh `@` token is never a drill; drop any stale anchor
                self.drill_prefix = None;
                // Entering @-mode always restarts the walk.
                self.start_query(
                    RestartWalk::Restart {
                        hidden: ctx.is_hidden_mode(),
                    },
                    ctx.matcher_query(),
                );
            }
            (Some(old), Some(new)) => {
                // Staying in @-mode only re-walks when hidden mode toggled.
                let restart = if old.is_hidden_mode() != new.is_hidden_mode() {
                    RestartWalk::Restart {
                        hidden: new.is_hidden_mode(),
                    }
                } else {
                    RestartWalk::Keep
                };
                self.start_query(restart, new.matcher_query());
            }
            (Some(_), None) => {
                // Leaving @-mode: clear results and the drill anchor.
                self.context = None;
                self.drill_prefix = None;
                self.shown_query = None;
                self.results = FuzzyMatcherDaemonResults::default();
                return;
            }
            (None, None) => return,
        }

        self.context = new_ctx;
        // Both @-mode arms build the daemon via `start_query`, so an active context implies a built daemon
        debug_assert!(self.context.is_none() || self.daemon.is_some());
    }

    /// Show `hits` for an `@` path that names a folder outside the workspace walk.
    fn install_outside_listing(&mut self, ctx: AtContext, hits: Vec<FuzzyMatchResult>) {
        let query = ctx.matcher_query().to_owned();
        let num_items = hits.len();
        self.min_generation = self.min_generation.saturating_add(1);
        self.outside_listing = true;
        self.shown_query = Some(query.clone());
        self.results = FuzzyMatcherDaemonResults {
            topk: Arc::from(hits),
            num_items,
            status: xai_grok_workspace::file_system::FuzzyMatcherStatus {
                done: true,
                changed: true,
            },
            generation: self.min_generation,
            query,
        };
        self.context = Some(ctx);
        self.selected = 0;
        self.hovered = None;
        self.scroll_offset = 0;
    }

    /// Clear the context (e.g., on Esc).
    pub fn clear_context(&mut self) {
        self.context = None;
        self.drill_prefix = None;
        self.shown_query = None;
        self.outside_listing = false;
        self.results = FuzzyMatcherDaemonResults::default();
    }

    /// Poll the daemon for new results. Returns `true` if results changed.
    ///
    /// Call this on every tick (~4ms) while the dropdown is potentially visible.
    pub fn poll(&mut self) -> bool {
        #[cfg(test)]
        {
            self.poll_calls += 1;
        }
        if self.context.is_none() || self.outside_listing {
            return false;
        }

        // Never build the daemon on the poll path: no daemon means no results yet.
        let Some(daemon) = self.daemon.as_ref() else {
            return false;
        };
        let results = daemon.get();

        if Arc::ptr_eq(&results.topk, &self.results.topk) {
            return false;
        }

        let Some(current_query) = self
            .context
            .as_ref()
            .map(|ctx| ctx.matcher_query().to_owned())
        else {
            return false;
        };
        if !accept_polled_results(&current_query, self.min_generation, &results) {
            return false;
        }

        self.min_generation = results.generation;
        self.shown_query = Some(current_query);
        self.results = results;
        if !self.results.topk.is_empty() {
            self.selected = self.selected.min(self.results.topk.len() - 1);
        }
        true
    }

    /// Move selection by `delta` items (negative is up, positive is down).
    pub fn move_selection(&mut self, delta: isize) {
        if self.is_scoring() {
            return;
        }
        let len = self.results.topk.len();
        if len == 0 {
            return;
        }
        let max_idx = len - 1;
        let current = self.selected.min(max_idx);
        self.selected = (current as isize + delta).clamp(0, max_idx as isize) as usize;
    }

    /// Move selection by a page (half of visible height).
    pub fn page_move(&mut self, delta: isize, visible_rows: usize) {
        let half = (visible_rows / 2).max(1) as isize;
        self.move_selection(delta * half);
    }

    /// Ensure the selected item is visible in the dropdown viewport.
    pub fn ensure_visible(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected + 1 - visible_rows;
        }
    }

    /// Select the hovered item (for click-to-accept).
    /// Returns `true` if there was a valid hovered item to select.
    pub fn select_hovered(&mut self) -> bool {
        if let Some(idx) = self.hovered
            && idx < self.results.topk.len()
        {
            self.selected = idx;
            return true;
        }
        false
    }

    /// Get the currently selected fuzzy match result.
    /// A hit scored for an earlier query is not selectable.
    pub fn selected_result(&self) -> Option<&FuzzyMatchResult> {
        if !self.hits_are_for_current_query() || self.is_scoring() {
            return None;
        }
        self.results.topk.get(self.selected)
    }

    /// A directory whose `/`-append matches text already present is committed (dismiss); otherwise the
    /// caller drills in and stays open.
    pub fn try_replace(&self, src: &str) -> Option<FileSearchReplacement> {
        if !self.hits_are_for_current_query() || self.is_scoring() {
            return None;
        }
        let ctx = self.context.as_ref()?;
        let res = self.results.topk.get(self.selected)?;

        // Dir-only contract: this always appends `/`, so it is valid only for a directory chosen in dir mode
        // Enforce it here so a file-selection caller can never emit `some/file.rs/`
        if !res.is_dir || !ctx.is_dir_mode() {
            return None;
        }

        // Replace only the path portion of the @-token (preserving `@` and any hidden-mode `!` marker)
        // See `AtContext::path_range`
        let range = ctx.path_range();
        let path = normalize_display_path(&res.path.to_string()).to_owned();
        let at_end = range.end == src.len();

        // A `/`-append that matches text already present commits the dir and dismisses; otherwise it drills in and stays open
        let no_op = src.get(range.clone()) == Some(accept_text(&path, false).as_str());
        let text = accept_text(&path, no_op && at_end);

        // Cursor sits just past the emitted text (after the trailing `/`).
        let mut cursor = range.start + text.len();
        // A committed dir that is not at the prompt end keeps its existing terminator (whitespace, `,`, or `;`, possibly multibyte)
        // See `context::detect`
        // Step past that one char so typing resumes after the directory
        if no_op && !at_end {
            cursor += src
                .get(range.end..)
                .and_then(|s| s.chars().next())
                .map_or(1, char::len_utf8);
        }

        Some(FileSearchReplacement {
            text,
            range,
            cursor,
            dismiss: no_op,
        })
    }

    /// Number of result items scored for the current query.
    pub fn result_count(&self) -> usize {
        if !self.hits_are_for_current_query() {
            return 0;
        }
        self.results.topk.len()
    }

    /// Total items the matcher knows about (for "k/n" display).
    pub fn total_items(&self) -> usize {
        self.results.num_items
    }

    /// Test-only: install a fake context and results snapshot so tests can drive acceptance flows without the background fuzzy daemon.
    ///
    /// Bumps `min_generation` past the seeded generation so any in-flight real daemon poll is rejected and cannot clobber the seeded state.
    #[cfg(test)]
    pub(crate) fn set_test_state(
        &mut self,
        context: AtContext,
        results: Vec<FuzzyMatchResult>,
        selected: usize,
    ) {
        let query = context.matcher_query().to_owned();
        self.shown_query = Some(query.clone());
        self.context = Some(context);
        self.results = FuzzyMatcherDaemonResults {
            topk: Arc::from(results),
            num_items: 0,
            status: xai_grok_workspace::file_system::FuzzyMatcherStatus {
                done: true,
                changed: false,
            },
            generation: self.min_generation,
            query,
        };
        self.min_generation += 1;
        self.selected = selected;
    }

    /// Test-only: mark the seeded snapshot as still scoring, or settled.
    #[cfg(test)]
    pub(crate) fn set_scoring_for_test(&mut self, scoring: bool) {
        self.results.status.done = !scoring;
    }

    /// Test-only observable state: whether the lazy daemon has been built yet.
    #[cfg(test)]
    pub(crate) fn daemon_is_built(&self) -> bool {
        self.daemon.is_some()
    }

    /// Test-only observable state: how many times the lazy daemon has been built.
    #[cfg(test)]
    pub(crate) fn daemon_build_count(&self) -> usize {
        self.daemon_builds
    }

    /// Test-only observable state: how many times [`Self::poll`] has run.
    #[cfg(test)]
    pub(crate) fn poll_calls(&self) -> u64 {
        self.poll_calls
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xai_grok_workspace::file_system::FuzzyMatcherStatus;

    fn dir_result(path: &str) -> FuzzyMatchResult {
        FuzzyMatchResult {
            path: nucleo::Utf32String::from(path),
            is_dir: true,
            ..Default::default()
        }
    }

    #[test]
    fn try_replace_commits_directory_already_present() {
        // The selected dir's `/`-append already matches the token text, so acceptance commits (dismiss) rather than drilling
        let mut state = FileSearchState::new(Path::new("."));

        // At the prompt end: append a trailing space so typing can continue.
        let src = "@src/";
        let ctx = context::detect(src, src.len()).expect("context");
        state.set_test_state(ctx, vec![dir_result("src")], 0);
        let r = state.try_replace(src).expect("replacement");
        assert!(r.dismiss);
        assert_eq!(r.range, 1..5);
        assert_eq!(r.text, "src/ ");
        assert_eq!(r.cursor, "@src/ ".len());

        // Mid-prompt: no appended space; step past the existing terminator.
        let src = "@src/ tail";
        let ctx = context::detect(src, 5).expect("context");
        state.set_test_state(ctx, vec![dir_result("src")], 0);
        let r = state.try_replace(src).expect("replacement");
        assert!(r.dismiss);
        assert_eq!(r.text, "src/");
        assert_eq!(r.cursor, 6);

        // Mid-prompt with a multibyte terminator: step past the whole char.
        let src = "@src/\u{a0}tail";
        let ctx = context::detect(src, 5).expect("context");
        state.set_test_state(ctx, vec![dir_result("src")], 0);
        let r = state.try_replace(src).expect("replacement");
        assert!(r.dismiss);
        assert_eq!(r.text, "src/");
        assert_eq!(r.cursor, 5 + '\u{a0}'.len_utf8());
    }

    #[test]
    fn retarget_drops_built_daemon() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut state = FileSearchState::new(dir.path());
        state.update_context("@alpha", "@alpha".len());
        assert!(state.daemon_is_built());

        state.retarget(Path::new(".."));
        assert_eq!(state.root(), Path::new(".."));
        assert!(!state.daemon_is_built());
    }

    #[test]
    fn poll_does_not_build_daemon() {
        let mut state = FileSearchState::new(Path::new("."));
        // With no @-context, poll returns early and never touches the daemon.
        assert!(!state.poll());
        assert!(!state.daemon_is_built());

        // With an @-context but an unbuilt daemon, poll must not force construction.
        let ctx = context::detect("@foo", 4).expect("context");
        state.set_test_state(ctx, Vec::new(), 0);
        assert!(state.context().is_some());
        assert!(!state.poll());
        assert!(!state.daemon_is_built());
    }

    #[test]
    fn daemon_is_built_lazily_on_first_use() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut state = FileSearchState::new(dir.path());
        assert!(!state.daemon_is_built());

        // The first @-search interaction builds the daemon lazily.
        state.update_context("@alpha", "@alpha".len());
        assert!(state.daemon_is_built());
        assert!(state.context().is_some());

        // A query edit stays in @-mode and reuses the same daemon: the build count stays at 1, proving no drop-and-rebuild
        assert_eq!(state.daemon_build_count(), 1);
        state.update_context("@alpha_marker", "@alpha_marker".len());
        assert!(state.daemon_is_built());
        assert_eq!(state.daemon_build_count(), 1);
    }

    fn file_result(path: &str) -> FuzzyMatchResult {
        FuzzyMatchResult {
            path: nucleo::Utf32String::from(path),
            is_dir: false,
            ..Default::default()
        }
    }

    #[test]
    fn typing_a_longer_query_drops_the_earlier_match() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut state = FileSearchState::new(dir.path());
        let src = "@a";
        let ctx = context::detect(src, src.len()).expect("context");
        state.set_test_state(ctx, vec![file_result("alpha.rs")], 0);
        assert!(state.selected_result().is_some());

        state.update_context("@ab", "@ab".len());
        assert_eq!(state.result_count(), 0);
        assert!(state.selected_result().is_none());
        assert!(!state.is_visible());
    }

    #[test]
    fn same_query_keeps_the_match_scored_for_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut state = FileSearchState::new(dir.path());
        let src = "@ab";
        let ctx = context::detect(src, src.len()).expect("context");
        state.set_test_state(ctx, vec![file_result("ab.rs")], 0);

        state.update_context(src, src.len());
        assert_eq!(state.result_count(), 1);
        let hit = state.selected_result().expect("same query keeps the hit");
        assert_eq!(hit.path.to_string(), "ab.rs");
    }

    #[test]
    fn poll_rejects_hits_scored_for_an_earlier_query() {
        let stale = FuzzyMatcherDaemonResults {
            topk: Arc::from([file_result("alpha.rs")]),
            num_items: 1,
            status: FuzzyMatcherStatus {
                done: false,
                changed: true,
            },
            generation: 5,
            query: "a".to_owned(),
        };
        assert!(!accept_polled_results("ab", 4, &stale));

        let fresh = FuzzyMatcherDaemonResults {
            query: "ab".to_owned(),
            ..stale.clone()
        };
        assert!(accept_polled_results("ab", 4, &fresh));
        assert!(!accept_polled_results("ab", 6, &fresh));

        let pending_empty = FuzzyMatcherDaemonResults {
            query: "ab".to_owned(),
            generation: 5,
            status: FuzzyMatcherStatus {
                done: false,
                changed: false,
            },
            ..Default::default()
        };
        assert!(!accept_polled_results("ab", 4, &pending_empty));

        let stale_done = FuzzyMatcherDaemonResults {
            query: "a".to_owned(),
            generation: 5,
            status: FuzzyMatcherStatus {
                done: true,
                changed: false,
            },
            ..Default::default()
        };
        assert!(!accept_polled_results("ab", 4, &stale_done));
    }

    #[test]
    fn scoring_does_not_move_the_highlight_or_return_a_hit() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut state = FileSearchState::new(dir.path());
        let src = "@ab";
        let ctx = context::detect(src, src.len()).expect("context");
        state.set_test_state(ctx, vec![file_result("a.rs"), file_result("b.rs")], 0);
        state.set_scoring_for_test(true);
        state.move_selection(1);
        assert_eq!(state.selected(), 0);
        assert!(!state.set_hovered(Some(1)));
        assert_eq!(state.hovered(), None);
        assert!(state.selected_result().is_none());

        state.set_scoring_for_test(false);
        state.move_selection(1);
        assert_eq!(state.selected(), 1);
        let hit = state
            .selected_result()
            .expect("settled results are selectable");
        assert_eq!(hit.path.to_string(), "b.rs");
    }

    fn listed_paths(state: &FileSearchState) -> Vec<String> {
        state
            .results()
            .topk
            .iter()
            .map(|hit| hit.path.to_string())
            .collect()
    }

    #[test]
    fn at_paths_outside_the_workspace_list_that_folder() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let workspace = tmp.path().join("workspace");
        let sibling = tmp.path().join("sibling");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(&workspace).expect("workspace");
        std::fs::create_dir_all(&sibling).expect("sibling");
        std::fs::create_dir_all(&home).expect("home");
        std::fs::write(workspace.join("inside.txt"), b"in").expect("inside");
        std::fs::write(sibling.join("outside.txt"), b"out").expect("outside");
        std::fs::write(sibling.join(".secret"), b"hid").expect("secret");
        std::fs::write(home.join("notes.txt"), b"note").expect("notes");

        let mut state = FileSearchState::new(&workspace);
        state.set_home_dir(home);
        assert!(
            xai_grok_shell::agent::config::UiConfig::default()
                .at_picker_outside_workspace_enabled()
        );

        let parent = "@../sibling/";
        state.update_context(parent, parent.len());
        let listed = listed_paths(&state);
        assert!(
            listed.iter().any(|path| path == "../sibling/outside.txt"),
            "parent folder should list outside.txt, got {listed:?}"
        );
        assert!(
            listed.iter().all(|path| !path.ends_with(".secret")),
            "dotfiles stay hidden, got {listed:?}"
        );
        assert!(listed.iter().all(|path| !path.contains("inside.txt")));

        let hidden = "@!../sibling/";
        state.update_context(hidden, hidden.len());
        let listed = listed_paths(&state);
        assert!(
            listed.iter().any(|path| path == "../sibling/.secret"),
            "hidden mode lists the dotfile, got {listed:?}"
        );

        let filtered = "@../sib";
        state.update_context(filtered, filtered.len());
        let listed = listed_paths(&state);
        assert!(
            listed.iter().any(|path| path == "../sibling"),
            "a partial name lists that folder, got {listed:?}"
        );

        let tilde = "@~/";
        state.update_context(tilde, tilde.len());
        let listed = listed_paths(&state);
        assert!(
            listed.iter().any(|path| path == "~/notes.txt"),
            "home folder should list notes.txt, got {listed:?}"
        );

        let absolute = format!("{}/", sibling.display());
        let absolute_query = format!("@{absolute}");
        state.update_context(&absolute_query, absolute_query.len());
        let listed = listed_paths(&state);
        let expected = format!("{absolute}outside.txt");
        assert!(
            listed.iter().any(|path| path == &expected),
            "absolute folder should list outside.txt, got {listed:?}"
        );
        assert!(!state.daemon_is_built());

        let off: xai_grok_shell::agent::config::UiConfig =
            toml::from_str("at_picker_outside_workspace = false").expect("ui config");
        assert!(!off.at_picker_outside_workspace_enabled());
        state.set_allow_outside_workspace(off.at_picker_outside_workspace_enabled());
        state.update_context(parent, parent.len());
        let listed = listed_paths(&state);
        assert!(
            listed.iter().all(|path| !path.contains("outside.txt")),
            "the switch keeps the picker inside the workspace, got {listed:?}"
        );
    }
}
