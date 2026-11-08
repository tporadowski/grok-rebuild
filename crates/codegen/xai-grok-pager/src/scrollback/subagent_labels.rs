//! Labels for subagent ids named in a finished reply.
//!
//! The scrollback stores the set on each entry. Rendering installs it for the duration of that entry's
//! `output()` so the agent-message block can swap the id for the label without a new `BlockContext` field.

use std::cell::RefCell;
use std::sync::Arc;

use crate::render::osc8::MIN_TRANSCRIPT_ID_LEN;

/// One id a finished reply may replace with a label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentLabel {
    pub(crate) id: Arc<str>,
    pub(crate) label: Arc<str>,
}

/// The labels active for one session, plus a generation so entry caches miss when the set changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentLabelSet {
    pub(crate) generation: u64,
    pub(crate) labels: Vec<SubagentLabel>,
}

impl SubagentLabelSet {
    pub(crate) fn empty() -> Self {
        Self {
            generation: 0,
            labels: Vec::new(),
        }
    }

    pub(crate) fn from_labels(mut labels: Vec<SubagentLabel>) -> Self {
        labels.retain(|label| linkable_id(&label.id) && !label.label.is_empty());
        labels.sort_by(|left, right| left.id.cmp(&right.id));
        labels.dedup_by(|left, right| left.id == right.id);
        let generation = if labels.is_empty() {
            0
        } else {
            fingerprint(&labels)
        };
        Self { generation, labels }
    }
}

fn fingerprint(labels: &[SubagentLabel]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for label in labels {
        for byte in label.id.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        for byte in label.label.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
    }
    hash.max(1)
}

fn linkable_id(id: &str) -> bool {
    id.len() >= MIN_TRANSCRIPT_ID_LEN
        && !id
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '(' | ')' | '\\'))
}

thread_local! {
    static ACTIVE: RefCell<Arc<SubagentLabelSet>> =
        RefCell::new(Arc::new(SubagentLabelSet::empty()));
}

pub(crate) struct LabelGuard {
    prev: Arc<SubagentLabelSet>,
}

impl Drop for LabelGuard {
    fn drop(&mut self) {
        ACTIVE.with(|slot| *slot.borrow_mut() = Arc::clone(&self.prev));
    }
}

pub(crate) fn install(set: Arc<SubagentLabelSet>) -> LabelGuard {
    ACTIVE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let prev = Arc::clone(&slot);
        *slot = set;
        LabelGuard { prev }
    })
}

pub(crate) fn active_labels() -> Arc<SubagentLabelSet> {
    ACTIVE.with(|slot| Arc::clone(&slot.borrow()))
}

fn id_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn whole_token(text: &str, at: usize, end: usize) -> bool {
    let before_ok = at == 0
        || text[..at]
            .chars()
            .next_back()
            .is_some_and(|ch| !id_token_char(ch));
    let after_ok = end >= text.len()
        || text[end..]
            .chars()
            .next()
            .is_some_and(|ch| !id_token_char(ch));
    before_ok && after_ok
}

fn is_fence_line(line: &str) -> bool {
    let trimmed = line.trim_start_matches(' ');
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return false;
    }
    let trimmed = trimmed.trim_end_matches('\r');
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn earliest_match<'a>(
    line: &str,
    labels: &'a [SubagentLabel],
) -> Option<(usize, usize, &'a SubagentLabel)> {
    let mut best: Option<(usize, usize, &SubagentLabel)> = None;
    for label in labels {
        let needle = label.id.as_ref();
        let mut from = 0;
        while from < line.len() {
            let Some(rel) = line[from..].find(needle) else {
                break;
            };
            let at = from + rel;
            let end = at + needle.len();
            if whole_token(line, at, end) {
                if best.as_ref().is_none_or(|(best_at, _, _)| at < *best_at) {
                    best = Some((at, end, label));
                }
                break;
            }
            from = at + 1;
        }
    }
    best
}

fn rewrite_line(line: &str, labels: &[SubagentLabel]) -> String {
    let mut out = String::new();
    let mut rest = line;
    while !rest.is_empty() {
        if rest.starts_with('`') {
            let ticks = rest.bytes().take_while(|byte| *byte == b'`').count();
            let marker = &rest[..ticks];
            out.push_str(marker);
            rest = &rest[ticks..];
            if let Some(end) = rest.find(marker) {
                let close = end + ticks;
                out.push_str(&rest[..close]);
                rest = &rest[close..];
            } else {
                out.push_str(rest);
                rest = "";
            }
            continue;
        }
        let Some((at, end, label)) = earliest_match(rest, labels) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..at]);
        out.push_str(&label.label);
        rest = &rest[end..];
    }
    out
}

/// Replace whole-token subagent ids outside fenced and inline code with a markdown link whose text is the label.
pub(crate) fn rewrite_finished_reply(source: &str, labels: &[SubagentLabel]) -> String {
    if labels.is_empty() || source.is_empty() {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut in_fence = false;
    for line in source.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let ending = &line[body.len()..];
        if is_fence_line(body) {
            in_fence = !in_fence;
            out.push_str(line);
            continue;
        }
        if in_fence {
            out.push_str(line);
            continue;
        }
        out.push_str(&rewrite_line(body, labels));
        out.push_str(ending);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(id: &str, text: &str) -> SubagentLabel {
        SubagentLabel {
            id: Arc::from(id),
            label: Arc::from(text),
        }
    }

    #[test]
    fn rewrite_replaces_a_prose_id_and_leaves_code_and_glued_tokens() {
        let id = "019f972a-7c1b-7abc-a896-4f08f91b9999";
        let source = format!("asked {id} now\n\n```\n{id}\n```\n\n`{id}` and x{id}y\n");
        let rewritten = rewrite_finished_reply(&source, &[label(id, "Subagent “ports”")]);
        assert!(rewritten.contains("asked Subagent “ports” now"));
        assert!(!rewritten.contains("grok-subagent:"));
        assert!(rewritten.contains(&format!("```\n{id}\n```")));
        assert!(rewritten.contains(&format!("`{id}`")));
        assert!(rewritten.contains(&format!("x{id}y")));
        assert!(!rewritten.contains(&format!("asked {id}")));
    }
}
