use anyhow::Result;

fn outln(line: impl std::fmt::Display) -> Result<()> {
    // `println!` panics when stdout closes (`grok sessions | head`) or the device is full.
    Ok(crate::util::println_stdout(line)?)
}
use clap::Subcommand;
use xai_grok_login::{AuthManager, try_ensure_fresh_auth};
use xai_grok_shell::agent::config::Config as AgentConfig;
use xai_grok_shell::session::merge::MergedSession;
use xai_grok_shell::util::grok_home::grok_home;
#[derive(Debug, clap::Args, Clone)]
pub struct SessionsArgs {
    #[command(subcommand)]
    command: SessionsCommand,
}

#[derive(Debug, Subcommand, Clone)]
enum SessionsCommand {
    /// List recent sessions (same as search with no query)
    List {
        /// Maximum number of sessions to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
    /// Search sessions by keyword
    Search {
        /// Search query (searches summaries and first prompts).
        query: String,
        /// Maximum number of sessions to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
    /// Permanently delete a session from history
    Delete {
        /// Session id to delete.
        id: String,
    },
}

pub async fn run(args: SessionsArgs, agent_config: &AgentConfig) -> Result<()> {
    // Best-effort only: never force an interactive public login here. Enterprise deployments may configure only a
    // deployment_key and a custom xai_api_base_url. Otherwise we still proceed so the SessionRegistryClient can use
    // the deployment_key when talking to the custom proxy.
    let auth = try_ensure_fresh_auth(
        &agent_config.grok_com_config,
        agent_config.endpoints.proxy_url(),
    )
    .await;

    let auth_manager = std::sync::Arc::new(AuthManager::new_with_proxy_base_url(
        &grok_home(),
        agent_config.grok_com_config.clone(),
        agent_config.endpoints.proxy_url(),
    ));

    let client = xai_grok_shell::agent::session_registry_client::SessionRegistryClient::new(
        agent_config.endpoints.proxy_url(),
        String::new(),
    )
    .with_deployment_key(agent_config.endpoints.deployment_key.clone())
    .with_alpha_test_key(agent_config.endpoints.alpha_test_key.clone())
    .with_auth(auth_manager.clone());

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());

    match args.command {
        SessionsCommand::List { limit } => {
            let sessions = xai_grok_shell::session::merge::fetch_merged(
                Some(&client),
                cwd.to_str(),
                xai_grok_shell::session::merge::CwdScope::WithSiblings,
                None,
                limit,
                // The CLI listing is an inventory, not the resume picker.
                xai_grok_shell::session::visibility::HeadlessPolicy::Include,
            )
            .await;
            print_sessions_grouped(&sessions)?;
        }
        SessionsCommand::Search { query, limit } => {
            use std::collections::HashSet;
            use xai_grok_shell::session::merge::REMOTE_TIMEOUT;
            use xai_grok_shell::session::storage::search::{
                IndexDecision, SessionSearchRequest, execute_search,
            };

            // Search is the only subcommand that reads the index, so it is the only one to start one
            let search = xai_grok_shell::session::storage::search::start_if_enabled(agent_config);

            let req = SessionSearchRequest {
                query,
                cwd: Some(cwd.to_string_lossy().to_string()),
                limit,
                offset: 0,
                include_content: true,
            };
            let root = grok_home();

            let remote_limit = (limit * 3).max(100) as i64;
            let (local_resp, remote_results) = tokio::join!(
                execute_search(IndexDecision::settled(&search), &root, &req),
                async {
                    tokio::time::timeout(
                        REMOTE_TIMEOUT,
                        client.search(Some(&req.query), remote_limit),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        eprintln!("warning: remote session search timed out");
                        Ok(Vec::new())
                    })
                    .unwrap_or_else(|e| {
                        eprintln!("warning: remote session search failed: {e}");
                        Vec::new()
                    })
                }
            );

            let resp = local_resp?;
            if let Some(by) = search.off_reason() {
                eprintln!(
                    "warning: local session search is off ({by}); searched remote sessions only."
                );
            }
            let local_ids: HashSet<&str> =
                resp.results.iter().map(|r| r.session_id.as_str()).collect();

            for hit in &resp.results {
                let title = summary_or_untitled(&hit.title);
                let time = chrono::DateTime::from_timestamp(hit.updated_at_unix, 0)
                    .map(|dt| {
                        dt.with_timezone(&chrono::Local)
                            .format("%b %d, %l:%M%P")
                            .to_string()
                    })
                    .unwrap_or_default();
                outln(format!(
                    "{} (score: {:.2})  {}\n  {}\n  {}",
                    hit.session_id,
                    hit.score,
                    time,
                    title,
                    hit.snippet.as_deref().unwrap_or("")
                ))?;
            }

            let remaining = limit.saturating_sub(resp.results.len());
            let mut remote_shown = 0usize;
            for r in &remote_results {
                if remote_shown >= remaining {
                    break;
                }
                if local_ids.contains(r.session_id.as_str()) {
                    continue;
                }
                let title = summary_or_untitled(&r.summary);
                let time = chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                    .map(|dt| {
                        dt.with_timezone(&chrono::Local)
                            .format("%b %d, %l:%M%P")
                            .to_string()
                    })
                    .unwrap_or_default();
                let snippet: String = r
                    .first_prompt
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(80)
                    .collect();
                outln(format!(
                    "{} (remote)  {}\n  {}\n  {}",
                    r.session_id, time, title, snippet
                ))?;
                remote_shown += 1;
            }

            outln(format!("\nTotal: {}", resp.results.len() + remote_shown))?;
        }
        SessionsCommand::Delete { id } => {
            // Always attempt the remote delete when authenticated and not ZDR; `list` and `search` likewise query remote
            // unconditionally. Gating on storage mode is impossible here: the CLI builds config without remote settings. ZDR
            // teams never upload, so there is nothing remote to delete.
            let needs_remote = auth.as_ref().is_some_and(|a| !a.is_zdr_team());

            // Pass `cwd = None` so the session is found by id regardless of which workspace it was created in
            // The local delete still uses the resolved per-session cwd
            // No search handle: the eviction inside prunes the row from another process's index, so a delete never needs one of its own
            let deletion = xai_grok_shell::session::persistence::delete_session_history(
                &id,
                None,
                needs_remote,
                auth_manager.clone(),
                None,
            )
            .await?;

            if deletion.any_removed() {
                outln(format!("Deleted session {id}"))?;
            } else {
                outln(format!("No session found with id {id}."))?;
            }
        }
    }

    Ok(())
}

/// The placeholder every session listing shows for a blank title
pub(crate) fn summary_or_untitled(title: &str) -> &str {
    if title.trim().is_empty() {
        "(untitled)"
    } else {
        title
    }
}

const SESSION_ID_WIDTH: usize = 36;
const DATE_WIDTH: usize = 10;
const STATUS_WIDTH: usize = 10;
const SUMMARY_WIDTH: usize = 50;

/// First `DATE_WIDTH` bytes of an ISO timestamp, which is the calendar date.
/// A slice that would split a character keeps the original string.
fn session_date_cell(ts: &str) -> &str {
    ts.get(..ts.len().min(DATE_WIDTH)).unwrap_or(ts)
}

/// One sessions-table row. Cells that would paint past their column end in `…`
/// and stay inside that many terminal columns. The date prefix stays a cut of
/// the timestamp, with no ellipsis, so `2026-11-13T12:00:00Z` still reads `2026-11-13`.
fn format_session_row(
    session_id: &str,
    created_at: &str,
    updated_at: &str,
    source: &str,
    summary: &str,
) -> String {
    format!(
        "{}  {}  {}  {}  {}",
        crate::util::truncate_to_width(session_id, SESSION_ID_WIDTH),
        session_date_cell(created_at),
        session_date_cell(updated_at),
        crate::util::truncate_to_width(source, STATUS_WIDTH),
        crate::util::truncate_to_width(summary, SUMMARY_WIDTH),
    )
}

/// Print sessions grouped by worktree label, preserving the original table format with a `Label: <label>` header before each group.
fn print_sessions_grouped(sessions: &[MergedSession]) -> Result<()> {
    if sessions.is_empty() {
        return outln("No sessions found.");
    }

    // Group by worktree_label, sort alphabetically, None last.
    let mut groups: std::collections::BTreeMap<Option<&str>, Vec<&MergedSession>> =
        std::collections::BTreeMap::new();
    for s in sessions {
        groups
            .entry(s.worktree_label.as_deref())
            .or_default()
            .push(s);
    }

    let header = format!(
        "{:<SESSION_ID_WIDTH$}  {:<DATE_WIDTH$}  {:<DATE_WIDTH$}  {:<STATUS_WIDTH$}  {}",
        "SESSION ID", "CREATED", "UPDATED", "STATUS", "SUMMARY"
    );

    // Labeled groups first (alphabetical), then unlabeled last.
    let none_group = groups.remove(&None);
    let print_group = |label_line: &str, members: &[&MergedSession]| -> Result<()> {
        outln(format!("\n{label_line}"))?;
        outln(&header)?;
        for s in members {
            let first_line;
            let summary: &str = if !s.summary.is_empty() {
                &s.summary
            } else if let Some(ref fp) = s.first_prompt
                && let Some(line) = fp.lines().find(|l| !l.trim().is_empty())
            {
                first_line = line.trim().to_string();
                &first_line
            } else {
                "(no summary)"
            };
            outln(format_session_row(
                &s.session_id,
                &s.created_at,
                &s.updated_at,
                &s.source,
                summary,
            ))?;
        }
        Ok(())
    };

    for (label, members) in &groups {
        let line = format!("Label: {}", label.unwrap_or(""));
        print_group(&line, members)?;
    }
    if let Some(members) = &none_group {
        print_group("(no label)", members)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary_cell(row: &str) -> &str {
        row.rsplit_once("  ")
            .map(|(_, summary)| summary)
            .unwrap_or(row)
    }

    #[test]
    fn session_summary_stays_within_display_columns_and_uses_one_ellipsis() {
        let ascii = format_session_row(
            "id",
            "2026-11-13T12:00:00Z",
            "2026-11-13T12:00:00Z",
            "local",
            "hello",
        );
        assert!(ascii.contains("2026-11-13"));
        assert!(!ascii.contains('…'));
        assert!(ascii.ends_with("hello"));

        let fifty = "a".repeat(SUMMARY_WIDTH);
        let exact = format_session_row("id", "2026-11-13", "2026-11-13", "local", &fifty);
        assert!(summary_cell(&exact) == fifty.as_str());

        let wide = "日本語".repeat(20);
        let row = format_session_row("id", "2026-11-13", "2026-11-13", "both", &wide);
        let summary = summary_cell(&row);
        assert!(summary.ends_with('…'), "{summary}");
        assert!(crate::util::display_columns(summary) <= SUMMARY_WIDTH);
        assert!(!summary.contains(&wide));

        let warning = format!("{}abcd", "\u{26A0}\u{FE0F}".repeat(SUMMARY_WIDTH));
        let emoji_row = format_session_row("id", "d", "d", "local", &warning);
        let emoji = summary_cell(&emoji_row);
        assert!(emoji.ends_with('…'), "{emoji}");
        assert!(crate::util::display_columns(emoji) <= SUMMARY_WIDTH);
    }

    #[test]
    fn session_id_and_status_cut_with_the_same_ellipsis() {
        let id = format_session_row(&"あ".repeat(40), "2026-11-13", "2026-11-13", "remote", "s");
        let shown_id = id.split("  ").next().unwrap_or("");
        assert!(shown_id.ends_with('…'), "{shown_id}");
        assert!(crate::util::display_columns(shown_id) <= SESSION_ID_WIDTH);

        let status = format_session_row("id", "2026-11-13", "2026-11-13", "ステータス値", "s");
        let cells: Vec<&str> = status.split("  ").collect();
        let source = cells.get(3).copied().unwrap_or("");
        assert!(source.ends_with('…'), "{status}");
        assert!(crate::util::display_columns(source) <= STATUS_WIDTH);
    }
}
