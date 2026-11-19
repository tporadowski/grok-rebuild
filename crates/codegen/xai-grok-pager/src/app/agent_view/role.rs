//! What an [`AgentView`] is: the root of its session, or a child mirrored from a subagent.
//! The role is set once, by [`AgentView::insert_subagent_view`]; every child-specific read site
//! derives its answer from it here instead of keeping a second flag that could drift.

use crate::app::actions::Action;
use crate::app::agent_view::AgentView;
use agent_client_protocol as acp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentRole {
    Root,
    Child(ChildLink),
}

/// How a child view relates to the session that spawned it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChildLink {
    /// The parent session id from the wire; the only session a message to this child may be sent on.
    parent_session_id: acp::SessionId,
    messaging: ChildMessaging,
}

impl ChildLink {
    /// A child with no `agentAddress` on the wire (replayed spawn, workflow child): nothing can be sent to it.
    pub(crate) fn unaddressable(parent_session_id: acp::SessionId) -> Self {
        Self {
            parent_session_id,
            messaging: ChildMessaging::Unaddressable,
        }
    }

    /// A live child the parent can send a note to. The address is the opaque `agentAddress`.
    pub(crate) fn addressable(
        parent_session_id: acp::SessionId,
        agent_address: impl Into<String>,
    ) -> Self {
        Self {
            parent_session_id,
            messaging: ChildMessaging::Addressable {
                agent_address: agent_address.into(),
            },
        }
    }

    pub(crate) fn parent_session_id(&self) -> &acp::SessionId {
        &self.parent_session_id
    }

    pub(crate) fn agent_address(&self) -> Option<&str> {
        match &self.messaging {
            ChildMessaging::Addressable { agent_address } => Some(agent_address.as_str()),
            ChildMessaging::Unaddressable => None,
        }
    }
}

/// Whether the pager can address the child at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChildMessaging {
    Unaddressable,
    Addressable { agent_address: String },
}

/// Which chrome the view paints: its own session's, or the framed takeover a parent draws around a child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewSurface {
    Root,
    ChildTakeover,
}

/// Where the composer's text would go, which decides whether it renders at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComposerRoute {
    RootSession,
    /// A note to an addressable subagent. Slash-like text is the note, not a pager command.
    Note,
    /// No route exists, so the composer gets zero rows and can't take focus.
    Hidden,
}

impl AgentView {
    /// `None` for the root of a session.
    pub(crate) fn child_link(&self) -> Option<&ChildLink> {
        match &self.role {
            AgentRole::Root => None,
            AgentRole::Child(link) => Some(link),
        }
    }

    pub(crate) fn surface(&self) -> ViewSurface {
        match &self.role {
            AgentRole::Root => ViewSurface::Root,
            AgentRole::Child(_) => ViewSurface::ChildTakeover,
        }
    }

    /// A note from this view. `None` unless the composer is the subagent note box.
    /// The text is kept whole, including a leading slash.
    pub(crate) fn subagent_note_action(&self, text: &str) -> Option<Action> {
        if self.composer_route() != ComposerRoute::Note {
            return None;
        }
        let link = self.child_link()?;
        let agent_address = link.agent_address()?.to_owned();
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        Some(Action::SendSubagentNote {
            parent_session_id: link.parent_session_id().clone(),
            agent_address,
            text: text.to_owned(),
        })
    }

    pub(crate) fn composer_route(&self) -> ComposerRoute {
        match &self.role {
            AgentRole::Root => ComposerRoute::RootSession,
            AgentRole::Child(link) if link.agent_address().is_some() => ComposerRoute::Note,
            AgentRole::Child(_) => ComposerRoute::Hidden,
        }
    }
}

#[cfg(test)]
#[path = "role_tests.rs"]
mod tests;
