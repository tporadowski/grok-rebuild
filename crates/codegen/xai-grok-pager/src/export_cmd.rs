use std::io::Write;
use std::path::{Path, PathBuf};

use agent_client_protocol as acp;
use anyhow::{Context, Result};

use crate::acp::meta::NotificationMeta;
use crate::acp::tracker::AcpUpdateTracker;
use crate::scrollback::export::render_blocks_to_markdown;
use crate::scrollback::{RenderBlock, ScrollbackState, ToolCallBlock};

#[derive(Debug, clap::Args, Clone)]
pub struct ExportArgs {
    /// Session ID to export
    pub session_id: String,
    /// Output file path (default: stdout)
    pub output: Option<PathBuf>,
    /// Copy to clipboard instead of writing to stdout
    #[arg(long, short)]
    pub clipboard: bool,
}

pub fn run(args: ExportArgs) -> Result<()> {
    tracing::info!(session_id = %args.session_id, "export_cmd: starting session export");

    let home = xai_grok_shell::util::grok_home::grok_home();
    let Some(session_dir) =
        xai_grok_shell::session::persistence::find_session_dir_by_id(&args.session_id)
    else {
        anyhow::bail!("Session '{}' not found.", args.session_id);
    };
    if !session_dir.join("summary.json").is_file() || !session_dir.join("updates.jsonl").is_file() {
        anyhow::bail!("Session '{}' not found.", args.session_id);
    }

    let loaded = load_export(&args.session_id, &home)?;
    let md = loaded.markdown;

    if md.is_empty() {
        anyhow::bail!(
            "Session '{}' has no conversation content to export",
            args.session_id
        );
    }

    if let Some(path) = args.output {
        let expanded = PathBuf::from(shellexpand::tilde(&path.to_string_lossy()).as_ref());
        if let Some(parent) = expanded.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        crate::clipboard::write_replacing_text_file(&md, &expanded)
            .with_context(|| format!("Failed to write {}", expanded.display()))?;
        tracing::info!(
            session_id = %args.session_id,
            path = %expanded.display(),
            bytes = md.len(),
            "export_cmd: wrote transcript to file"
        );
        eprintln!("Conversation exported to {}", expanded.display());
    } else if args.clipboard {
        let _ = crate::clipboard::copy_text(&md);
        let lines = md.lines().count();
        tracing::info!(
            session_id = %args.session_id,
            bytes = md.len(),
            lines,
            "export_cmd: copied transcript to clipboard"
        );
        eprintln!(
            "Conversation copied to clipboard ({} chars, {} lines)",
            md.len(),
            lines
        );
    } else {
        let mut stdout = std::io::stdout().lock();
        crate::util::ignore_broken_pipe(stdout.write_all(md.as_bytes()))?;
        crate::util::ignore_broken_pipe(stdout.write_all(b"\n"))?;
    }

    Ok(())
}

struct ExportLoad {
    markdown: String,
    updates_applied: usize,
    retained_shell_bytes: usize,
}

/// Replay one session for export.
///
/// Session resume already drops in-progress bash `tool_call_update` lines before
/// deserializing `content` / `rawOutput`. Export used to materialize every line,
/// including those cumulative snapshots, then copy the finished stdout into the
/// scrollback. The markdown renderer only prints the command, so that copy is dropped.
fn load_export(session_id: &str, grok_home: &Path) -> Result<ExportLoad> {
    let mut tracker = AcpUpdateTracker::new();
    let mut scrollback = ScrollbackState::new();
    let replay_meta = NotificationMeta {
        is_replay: true,
        ..Default::default()
    };
    let mut updates_applied = 0usize;
    let emission =
        xai_grok_shell::session::storage::stream_replay_updates_at(session_id, grok_home, |update| {
            updates_applied += 1;
            tracker.handle_update(
                drop_export_shell_transcript(update),
                &replay_meta,
                &mut scrollback,
            );
        })
        .with_context(|| format!("failed to read session '{session_id}'"))?;

    let markdown = render_blocks_to_markdown(
        (0..scrollback.len()).filter_map(|index| scrollback.entry(index).map(|entry| &entry.block)),
    );
    let retained_shell_bytes = retained_shell_bytes(&scrollback);
    tracing::debug!(
        session_id,
        updates_applied,
        retained_shell_bytes,
        ?emission,
        bytes = markdown.len(),
        "export_cmd: replayed session"
    );
    Ok(ExportLoad {
        markdown,
        updates_applied,
        retained_shell_bytes,
    })
}

fn drop_export_shell_transcript(update: acp::SessionUpdate) -> acp::SessionUpdate {
    match update {
        acp::SessionUpdate::ToolCall(tc)
            if tc.kind == acp::ToolKind::Execute || raw_output_is_shell(&tc.raw_output) =>
        {
            acp::SessionUpdate::ToolCall(without_shell_payload(tc))
        }
        acp::SessionUpdate::ToolCallUpdate(tcu)
            if tcu.fields.kind == Some(acp::ToolKind::Execute)
                || raw_output_is_shell(&tcu.fields.raw_output) =>
        {
            acp::SessionUpdate::ToolCallUpdate(without_shell_payload_update(tcu))
        }
        other => other,
    }
}

fn without_shell_payload(mut tc: acp::ToolCall) -> acp::ToolCall {
    tc.content.clear();
    tc.raw_output = None;
    tc
}

fn without_shell_payload_update(mut update: acp::ToolCallUpdate) -> acp::ToolCallUpdate {
    update.fields.content = Some(Vec::new());
    update.fields.raw_output = None;
    update
}

fn raw_output_is_shell(raw: &Option<serde_json::Value>) -> bool {
    raw.as_ref()
        .and_then(|value| value.get("type"))
        .and_then(|tag| tag.as_str())
        == Some("Bash")
}

fn retained_shell_bytes(scrollback: &ScrollbackState) -> usize {
    (0..scrollback.len())
        .filter_map(|index| scrollback.entry(index))
        .map(|entry| match &entry.block {
            RenderBlock::ToolCall(ToolCallBlock::Execute(execute)) => {
                execute.output.as_ref().map(String::len).unwrap_or(0)
                    + execute.error.as_ref().map(String::len).unwrap_or(0)
            }
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    use xai_grok_tools::types::output::{BashOutput, ToolOutput};

    fn envelope(update: acp::SessionUpdate) -> String {
        let notification = acp::SessionNotification::new(acp::SessionId::new("s"), update);
        serde_json::json!({
            "timestamp": 1,
            "method": "session/update",
            "params": notification,
        })
        .to_string()
    }

    fn bash_output(text: &str) -> serde_json::Value {
        serde_json::to_value(ToolOutput::Bash(BashOutput {
            output: text.as_bytes().to_vec(),
            output_for_prompt: text.to_string(),
            exit_code: 0,
            command: "echo hello".to_string(),
            truncated: false,
            signal: None,
            timed_out: false,
            description: None,
            current_dir: "/tmp".to_string(),
            output_file: String::new(),
            total_bytes: text.len(),
            output_delta: None,
            was_bare_echo: false,
        }))
        .expect("bash output serializes")
    }

    fn shell_update(status: acp::ToolCallStatus, text: &str) -> acp::SessionUpdate {
        acp::SessionUpdate::ToolCallUpdate(acp::ToolCallUpdate::new(
            acp::ToolCallId::new("t1"),
            acp::ToolCallUpdateFields::new()
                .status(Some(status))
                .content(Some(vec![acp::ToolCallContent::from(
                    acp::ContentBlock::Text(acp::TextContent::new(text)),
                )]))
                .raw_output(bash_output(text)),
        ))
    }

    /// Cumulative in-progress snapshots stay on disk. Export must not apply them
    /// or keep the finished stdout: the markdown line is only the command.
    #[test]
    fn export_of_large_shell_output_skips_snapshots_and_drops_stdout() {
        let home = tempfile::tempdir().expect("temp grok home");
        let session_id = "sess-export";
        let dir = home
            .path()
            .join("sessions")
            .join("cwd-bucket")
            .join(session_id);
        std::fs::create_dir_all(&dir).expect("session dir");
        std::fs::write(dir.join("summary.json"), "{}").expect("summary");

        let chunk = "CHUNK_SENTINEL ".repeat(256);
        let stdout = "FINAL_STDOUT ".repeat(512);
        let user = envelope(acp::SessionUpdate::UserMessageChunk(
            acp::ContentChunk::new(acp::ContentBlock::Text(acp::TextContent::new(
                "please list",
            ))),
        ));
        let agent = envelope(acp::SessionUpdate::AgentMessageChunk(
            acp::ContentChunk::new(acp::ContentBlock::Text(acp::TextContent::new(
                "done listing",
            ))),
        ));
        let start = envelope(acp::SessionUpdate::ToolCall(
            acp::ToolCall::new(acp::ToolCallId::new("t1"), "Execute `echo hello`")
                .kind(acp::ToolKind::Execute)
                .raw_input(serde_json::json!({
                    "command": "echo hello",
                    "description": "say hi",
                })),
        ));
        let mut lines = vec![user, agent, start];
        for _ in 0..3 {
            lines.push(envelope(shell_update(
                acp::ToolCallStatus::InProgress,
                &chunk,
            )));
        }
        lines.push(envelope(shell_update(
            acp::ToolCallStatus::Completed,
            &stdout,
        )));
        std::fs::write(dir.join("updates.jsonl"), lines.join("\n")).expect("updates");

        let loaded = load_export(session_id, home.path()).expect("export");
        assert_eq!(
            loaded.updates_applied, 3,
            "user, assistant, and one collapsed tool call; three shell snapshots stay unread"
        );
        assert_eq!(
            loaded.retained_shell_bytes, 0,
            "export keeps the command, not the shell transcript"
        );
        assert!(
            loaded.markdown.contains("please list"),
            "user text missing: {}",
            loaded.markdown
        );
        assert!(
            loaded.markdown.contains("done listing"),
            "assistant text missing: {}",
            loaded.markdown
        );
        assert!(
            loaded.markdown.contains("Execute: echo hello (say hi)"),
            "command line missing: {}",
            loaded.markdown
        );
        assert!(
            !loaded.markdown.contains("CHUNK_SENTINEL"),
            "in-progress snapshot leaked into the export"
        );
        assert!(
            !loaded.markdown.contains("FINAL_STDOUT"),
            "finished stdout leaked into the export"
        );
    }
}
