//! OSC 7501 program-status reports.
//!
//! The focused session is the root record (the main status). With
//! `[ui.notifications.program_status] children = true` (the default), every
//! other open session and every running subagent is its own record. `children = false`
//! emits only the root record and clears the others.
//!
//! Wire format: <https://www.superlogical.com/rex/docs/build/program-status>.

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::app::agent_view::AgentView;

/// What the session is doing before the done/failed hold is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawActivity {
    Idle,
    Working,
    WaitingPermission,
    WaitingQuestion,
}

/// One open session, plus the subagents that are still running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionSnap {
    pub key: String,
    pub title: String,
    pub active: bool,
    pub raw: RawActivity,
    pub subagents: Vec<SubagentSnap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubagentSnap {
    pub key: String,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Activity {
    Idle,
    Working,
    WaitingPermission,
    WaitingQuestion,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    /// `None` is the root record.
    id: Option<String>,
    activity: Activity,
    title: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Hold {
    activity: Activity,
    failed: bool,
}

impl Default for Activity {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProgramStatusPublisher {
    holds: BTreeMap<String, Hold>,
    /// Record id (`""` is the root) to the last sequence written for it.
    emitted: BTreeMap<String, String>,
}

impl ProgramStatusPublisher {
    pub(crate) fn note_failed(&mut self, key: &str) {
        self.holds.entry(key.to_owned()).or_default().failed = true;
    }

    /// Sequences to write now. Empty when nothing changed.
    pub(crate) fn update(
        &mut self,
        sessions: &[SessionSnap],
        children: bool,
        focused: bool,
    ) -> Vec<String> {
        let settled = self.settle(sessions, focused);
        let records = records_from(&settled, children);
        self.diff(records)
    }

    /// `state=clear` with no id removes every record. Used on shutdown.
    pub(crate) fn clear_all(&mut self) -> String {
        self.holds.clear();
        self.emitted.clear();
        encode_clear(None)
    }

    fn settle(&mut self, sessions: &[SessionSnap], focused: bool) -> Vec<SettledSession> {
        let mut live = BTreeMap::new();
        let mut settled = Vec::with_capacity(sessions.len());
        for session in sessions {
            let mut hold = self.holds.get(&session.key).copied().unwrap_or_default();
            let activity = settle_one(session.raw, session.active, focused, &mut hold);
            live.insert(session.key.clone(), hold);
            let mut subagents = session.subagents.clone();
            subagents.sort_by(|a, b| a.key.cmp(&b.key));
            settled.push(SettledSession {
                key: session.key.clone(),
                title: session.title.clone(),
                active: session.active,
                activity,
                subagents,
            });
        }
        self.holds = live;
        settled
    }

    fn diff(&mut self, records: Vec<Record>) -> Vec<String> {
        let mut wanted = BTreeMap::new();
        for record in records {
            let key = record.id.clone().unwrap_or_default();
            wanted.insert(key, encode_record(&record));
        }
        let mut out = Vec::new();
        for (id, seq) in &wanted {
            if self.emitted.get(id) != Some(seq) {
                out.push(seq.clone());
            }
        }
        let stale: Vec<String> = self
            .emitted
            .keys()
            .filter(|id| !wanted.contains_key(*id))
            .cloned()
            .collect();
        for id in stale {
            let clear_id = if id.is_empty() {
                None
            } else {
                Some(id.as_str())
            };
            out.push(encode_clear(clear_id));
        }
        self.emitted = wanted;
        out
    }
}

struct SettledSession {
    key: String,
    title: String,
    active: bool,
    activity: Activity,
    subagents: Vec<SubagentSnap>,
}

fn settle_one(raw: RawActivity, active: bool, focused: bool, hold: &mut Hold) -> Activity {
    let activity = match raw {
        RawActivity::Working => Activity::Working,
        RawActivity::WaitingPermission => Activity::WaitingPermission,
        RawActivity::WaitingQuestion => Activity::WaitingQuestion,
        RawActivity::Idle if hold.failed => {
            if active && focused {
                hold.failed = false;
                Activity::Idle
            } else {
                Activity::Failed
            }
        }
        RawActivity::Idle
            if matches!(
                hold.activity,
                Activity::Working
                    | Activity::WaitingPermission
                    | Activity::WaitingQuestion
                    | Activity::Done
            ) =>
        {
            if active && focused {
                Activity::Idle
            } else {
                Activity::Done
            }
        }
        RawActivity::Idle => Activity::Idle,
    };
    if !matches!(raw, RawActivity::Idle) {
        hold.failed = false;
    }
    hold.activity = activity;
    activity
}

fn records_from(sessions: &[SettledSession], children: bool) -> Vec<Record> {
    let mut records = Vec::new();
    if let Some(active) = sessions.iter().find(|session| session.active) {
        records.push(session_record(active, None));
    } else {
        records.push(Record {
            id: None,
            activity: Activity::Idle,
            title: None,
        });
    }
    if !children {
        return records;
    }
    let mut rest: Vec<&SettledSession> =
        sessions.iter().filter(|session| !session.active).collect();
    rest.sort_by(|a, b| a.key.cmp(&b.key));
    for session in rest {
        records.push(session_record(session, Some(record_segment(&session.key))));
    }
    let mut subs = Vec::new();
    for session in sessions {
        let owner = record_segment(&session.key);
        for sub in &session.subagents {
            subs.push((
                format!("{owner}/{}", record_segment(&sub.key)),
                sub.title.clone(),
            ));
        }
    }
    subs.sort_by(|a, b| a.0.cmp(&b.0));
    for (id, title) in subs {
        let title = clean_line(&title);
        records.push(Record {
            id: Some(id),
            activity: Activity::Working,
            title: (!title.is_empty()).then_some(title),
        });
    }
    records
}

fn session_record(session: &SettledSession, id: Option<String>) -> Record {
    let title = clean_line(&session.title);
    Record {
        id,
        activity: session.activity,
        title: (!title.is_empty()).then_some(title),
    }
}

pub(crate) fn session_key(agent: &AgentView) -> String {
    agent
        .session
        .session_id
        .as_ref()
        .map(|id| id.0.to_string())
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| format!("agent-{}", agent.session.id.0))
}

pub(crate) fn snap_agent(agent: &AgentView, active: bool) -> SessionSnap {
    let mut subagents = Vec::new();
    for info in agent.subagent_sessions.values() {
        if !info.attempt.lifecycle.is_running() {
            continue;
        }
        let title = info.description.trim().to_owned();
        subagents.push(SubagentSnap {
            key: info.subagent_id.to_string(),
            title,
        });
    }
    subagents.sort_by(|a, b| a.key.cmp(&b.key));
    let title = agent
        .display_name
        .as_deref()
        .or(agent.generated_session_title.as_deref())
        .unwrap_or("")
        .trim()
        .to_owned();
    SessionSnap {
        key: session_key(agent),
        title,
        active,
        raw: raw_activity(agent),
        subagents,
    }
}

fn raw_activity(agent: &AgentView) -> RawActivity {
    if agent.question_view.is_some() {
        return RawActivity::WaitingQuestion;
    }
    if !agent.permission_queue.is_empty() || agent.plan_approval_view.is_some() {
        return RawActivity::WaitingPermission;
    }
    if agent.session.state.is_busy() && !agent.renders_parked() {
        RawActivity::Working
    } else {
        RawActivity::Idle
    }
}

/// A path segment of the OSC 7501 id grammar. UUIDs drop their hyphens so they fit in 32 bytes.
fn record_segment(raw: &str) -> String {
    if is_segment(raw) {
        return raw.to_owned();
    }
    let stripped: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '+'))
        .collect();
    if is_segment(&stripped) {
        return stripped;
    }
    format!("{:016x}", fnv1a64(raw))
}

fn is_segment(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 32
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '+' | '-'))
}

fn fnv1a64(raw: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn encode_record(record: &Record) -> String {
    let mut pairs = vec![format!("state={}", state_name(record.activity))];
    if let Some(id) = &record.id {
        pairs.push(format!("id={id}"));
    }
    if let Some(kind) = block_kind(record.activity) {
        pairs.push(format!("kind={kind}"));
    }
    pairs.push("app=grok".to_owned());
    if let Some(title) = record.title.as_deref().and_then(|text| b64_text(text, 192)) {
        pairs.push(format!("title={title}"));
    }
    if let Some(msg) = b64_text(message(record.activity), 2048) {
        pairs.push(format!("msg={msg}"));
    }
    wrap_osc(&pairs.join(":"))
}

fn encode_clear(id: Option<&str>) -> String {
    match id {
        Some(id) => wrap_osc(&format!("state=clear:id={id}")),
        None => wrap_osc("state=clear"),
    }
}

fn wrap_osc(body: &str) -> String {
    format!("\x1b]7501;{body}\x1b\\")
}

fn state_name(activity: Activity) -> &'static str {
    match activity {
        Activity::Idle => "idle",
        Activity::Working => "working",
        Activity::WaitingPermission | Activity::WaitingQuestion => "blocked",
        Activity::Done => "done",
        Activity::Failed => "error",
    }
}

fn block_kind(activity: Activity) -> Option<&'static str> {
    match activity {
        Activity::WaitingPermission => Some("permission"),
        Activity::WaitingQuestion => Some("question"),
        _ => None,
    }
}

fn message(activity: Activity) -> &'static str {
    match activity {
        Activity::Idle => "Idle",
        Activity::Working => "Working",
        Activity::WaitingPermission | Activity::WaitingQuestion => "Waiting for you",
        Activity::Done => "Done",
        Activity::Failed => "Failed",
    }
}

fn clean_line(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

fn b64_text(text: &str, max_decoded: usize) -> Option<String> {
    let cleaned = clean_line(text);
    if cleaned.is_empty() {
        return None;
    }
    let truncated = truncate_bytes(&cleaned, max_decoded);
    Some(STANDARD.encode(truncated.as_bytes()))
}

fn truncate_bytes(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent::AgentState;
    use crate::app::agent_view::test_fixtures::{self, make_agent};
    use agent_client_protocol as acp;
    use base64::Engine;

    fn uuid(fill: char) -> String {
        let hex = fill.to_string().repeat(32);
        format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }

    fn agent(session: &str, name: &str, state: AgentState) -> AgentView {
        let mut agent = make_agent();
        agent.session.session_id = Some(acp::SessionId::new(session));
        agent.display_name = Some(name.to_owned());
        agent.session.state = state;
        agent
    }

    fn bodies(escapes: &[String]) -> Vec<BTreeMap<String, String>> {
        escapes
            .iter()
            .map(|seq| {
                let body = seq
                    .strip_prefix("\x1b]7501;")
                    .and_then(|rest| rest.strip_suffix("\x1b\\"))
                    .unwrap_or_else(|| panic!("not an OSC 7501 report: {seq:?}"));
                body.split(':')
                    .map(|pair| {
                        let (key, value) = pair
                            .split_once('=')
                            .unwrap_or_else(|| panic!("bad pair {pair}"));
                        (key.to_owned(), value.to_owned())
                    })
                    .collect()
            })
            .collect()
    }

    fn text_field(report: &BTreeMap<String, String>, key: &str) -> Option<String> {
        report.get(key).map(|value| {
            let bytes = STANDARD
                .decode(value)
                .unwrap_or_else(|err| panic!("decode {key}: {err}"));
            String::from_utf8(bytes).expect("utf-8")
        })
    }

    #[test]
    fn children_report_each_open_session_and_running_subagent() {
        let mut main = agent(&uuid('a'), "Main", AgentState::TurnRunning);
        main.subagent_sessions.insert(
            "child-1".into(),
            test_fixtures::running_subagent_info("child-1"),
        );
        let mut finished = test_fixtures::running_subagent_info("child-done");
        finished.attempt.lifecycle.set_finished_for_test(true);
        main.subagent_sessions.insert("child-done".into(), finished);
        let other = agent(&uuid('b'), "Other", AgentState::Idle);
        let snaps = vec![snap_agent(&main, true), snap_agent(&other, false)];
        let mut publisher = ProgramStatusPublisher::default();
        let reports = bodies(&publisher.update(&snaps, true, true));

        let root = reports
            .iter()
            .find(|report| !report.contains_key("id"))
            .expect("root record");
        assert_eq!(root.get("state").map(String::as_str), Some("working"));
        assert_eq!(root.get("app").map(String::as_str), Some("grok"));
        assert_eq!(text_field(root, "title").as_deref(), Some("Main"));
        assert_eq!(text_field(root, "msg").as_deref(), Some("Working"));

        let other_id = record_segment(&uuid('b'));
        let other_report = reports
            .iter()
            .find(|report| report.get("id").map(String::as_str) == Some(other_id.as_str()))
            .expect("other session record");
        assert_eq!(other_report.get("state").map(String::as_str), Some("idle"));
        assert_eq!(text_field(other_report, "title").as_deref(), Some("Other"));

        let sub_id = format!(
            "{}/{}",
            record_segment(&uuid('a')),
            record_segment("sa-child-1")
        );
        let sub = reports
            .iter()
            .find(|report| report.get("id").map(String::as_str) == Some(sub_id.as_str()))
            .expect("running subagent record");
        assert_eq!(sub.get("state").map(String::as_str), Some("working"));
        assert_eq!(text_field(sub, "title").as_deref(), Some("test"));

        let finished_id = format!(
            "{}/{}",
            record_segment(&uuid('a')),
            record_segment("sa-child-done")
        );
        assert!(
            reports
                .iter()
                .all(|report| report.get("id").map(String::as_str) != Some(finished_id.as_str())),
            "a finished subagent is not a record"
        );
        assert_eq!(reports.len(), 3);
        assert!(publisher.update(&snaps, true, true).is_empty());
    }

    #[test]
    fn children_false_reports_only_the_main_status() {
        let main = agent(&uuid('a'), "Main", AgentState::TurnRunning);
        let other = agent(&uuid('b'), "Other", AgentState::Idle);
        let snaps = vec![snap_agent(&main, true), snap_agent(&other, false)];
        let only_main = bodies(&ProgramStatusPublisher::default().update(&snaps, false, true));
        assert_eq!(
            only_main.len(),
            1,
            "children = false is one record: {only_main:?}"
        );
        assert!(
            !only_main[0].contains_key("id"),
            "the main status is the root record"
        );
        assert_eq!(
            only_main[0].get("state").map(String::as_str),
            Some("working")
        );
        assert_eq!(text_field(&only_main[0], "title").as_deref(), Some("Main"));

        let mut with_children = ProgramStatusPublisher::default();
        let first = with_children.update(&snaps, true, true);
        assert!(first.len() > 1, "children on emits a record per session");
        let cleared = bodies(&with_children.update(&snaps, false, true));
        assert!(
            cleared
                .iter()
                .all(|report| report.get("state").map(String::as_str) == Some("clear")),
            "turning children off clears the extra records and leaves the root alone: {cleared:?}"
        );
        assert!(!cleared.is_empty());
    }

    #[test]
    fn unfocused_turn_end_reports_done_until_the_session_is_focused() {
        let mut publisher = ProgramStatusPublisher::default();
        let working = SessionSnap {
            key: "main".into(),
            title: "Main".into(),
            active: true,
            raw: RawActivity::Working,
            subagents: Vec::new(),
        };
        let _ = publisher.update(std::slice::from_ref(&working), false, false);
        let idle = SessionSnap {
            raw: RawActivity::Idle,
            ..working.clone()
        };
        let done = bodies(&publisher.update(std::slice::from_ref(&idle), false, false));
        assert_eq!(done[0].get("state").map(String::as_str), Some("done"));
        assert_eq!(text_field(&done[0], "msg").as_deref(), Some("Done"));
        let back = bodies(&publisher.update(std::slice::from_ref(&idle), false, true));
        assert_eq!(back[0].get("state").map(String::as_str), Some("idle"));
    }

    #[test]
    fn failed_turn_reports_error_until_the_session_is_focused() {
        let mut publisher = ProgramStatusPublisher::default();
        publisher.note_failed("main");
        let idle = SessionSnap {
            key: "main".into(),
            title: "Main".into(),
            active: true,
            raw: RawActivity::Idle,
            subagents: Vec::new(),
        };
        let failed = bodies(&publisher.update(std::slice::from_ref(&idle), false, false));
        assert_eq!(failed[0].get("state").map(String::as_str), Some("error"));
        assert_eq!(text_field(&failed[0], "msg").as_deref(), Some("Failed"));
        let seen = bodies(&publisher.update(std::slice::from_ref(&idle), false, true));
        assert_eq!(seen[0].get("state").map(String::as_str), Some("idle"));
    }

    #[test]
    fn permission_and_question_cards_are_blocked() {
        let mut permission = agent("sess-perm", "Perm", AgentState::TurnRunning);
        permission
            .permission_queue
            .push_back(test_fixtures::make_followup_permission_state());
        let mut question = agent("sess-q", "Ask", AgentState::TurnRunning);
        question.question_view = Some(crate::views::question_view::QuestionViewState::new(
            "call-q".into(),
            Vec::new(),
            crate::views::prompt_widget::StashedPrompt::default(),
        ));
        let snaps = vec![snap_agent(&permission, true), snap_agent(&question, false)];
        let reports = bodies(&ProgramStatusPublisher::default().update(&snaps, true, true));
        let root = reports
            .iter()
            .find(|report| !report.contains_key("id"))
            .unwrap();
        assert_eq!(root.get("state").map(String::as_str), Some("blocked"));
        assert_eq!(root.get("kind").map(String::as_str), Some("permission"));
        assert_eq!(text_field(root, "msg").as_deref(), Some("Waiting for you"));
        let child = reports
            .iter()
            .find(|report| report.contains_key("id"))
            .unwrap();
        assert_eq!(child.get("state").map(String::as_str), Some("blocked"));
        assert_eq!(child.get("kind").map(String::as_str), Some("question"));
    }

    #[test]
    fn children_false_loads_from_notifications_toml() {
        let raw: toml::Value =
            toml::from_str("[ui.notifications.program_status]\nchildren = false\n").expect("toml");
        let config = crate::notifications::load_notification_config(&raw);
        assert!(!config.program_status.children);
        let absent: toml::Value =
            toml::from_str("[ui.notifications]\nmethod = \"none\"\n").unwrap();
        assert!(
            crate::notifications::load_notification_config(&absent)
                .program_status
                .children
        );
    }
}
