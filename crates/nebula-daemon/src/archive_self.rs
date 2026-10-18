//! `nebula archive` from inside an agent session: the user asked the agent
//! to archive (close, put away) the session it runs in. Archiving kills the
//! PTY, and the model is mid-turn when it runs the command — archived on the
//! spot, it would die before its reply reached the user. So the request is
//! remembered here and carried out on the turn-end hook, the same signal a
//! `nebula worktree` relocation waits for.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use nebula_core::{AgentId, AgentStatus};

use crate::registry::Daemon;
use crate::status::HookEvent;

/// What nebula appends to Claude's system prompt so "archive this thread"
/// becomes one `nebula archive` call. Only when asked: archiving puts the
/// session out of sight, which no agent should decide on its own.
pub const CLAUDE_ARCHIVE_GUIDANCE: &str = "[nebula] Only when the user explicitly asks you to \
archive, close or put away this session or thread (\"archive this thread\", \"close this \
session\") — and never on your own initiative, not even when the work looks finished — run this \
shell command, exactly once:\n\n  nebula archive\n\nnebula archives this session as soon as your \
current turn ends, so finish your reply first: say in one line that the session will be archived, \
and make no further tool calls. The user can unarchive it from nebula. If the command fails, report \
the error.";

/// Whether `event` ends the agent's turn: Stop, the idle notification a
/// Stop-less end still fires, or the progress clear that is the only word
/// of a cancelled turn. A Bash hook from the same turn is not one.
pub(crate) fn ends_turn(event: &HookEvent) -> bool {
    match event {
        HookEvent::Stop => true,
        HookEvent::Notification { notification_type } => {
            notification_type.as_deref() == Some("idle_prompt")
        }
        HookEvent::Progress { busy } => !busy,
        _ => false,
    }
}

impl Daemon {
    /// `nebula archive`, run by the agent inside its own session: archive
    /// it when this turn ends. A session that is not mid-turn has no reply
    /// to wait for and is archived at once.
    pub fn archive_after_turn(self: &Arc<Self>, id: &AgentId) -> Result<()> {
        let agent = self.store.get_agent(id)?.context("agent not found")?;
        if agent.archived {
            bail!("this session is already archived");
        }
        let mid_turn = self.status_machines.lock().get(id).is_some_and(|m| {
            matches!(
                m.status(),
                AgentStatus::Running | AgentStatus::NeedsFeedback
            )
        });
        if !mid_turn {
            return self.archive_agent(id);
        }
        tracing::info!(agent = %id, "archiving when the turn ends");
        self.pending_archives.lock().insert(id.clone());
        Ok(())
    }

    /// The turn an agent ran `nebula archive` in has ended: archive it now.
    pub fn complete_pending_archive(self: &Arc<Self>, id: &AgentId, event: &HookEvent) {
        if !ends_turn(event) || !self.pending_archives.lock().remove(id) {
            return;
        }
        if let Err(e) = self.archive_agent(id) {
            tracing::warn!(agent = %id, error = %e, "archive at turn end failed");
        }
    }

    /// Archives still waiting on their turn to end, for an IN-PLACE
    /// RESTART to carry.
    pub fn pending_archives(&self) -> Vec<AgentId> {
        self.pending_archives.lock().iter().cloned().collect()
    }

    pub fn restore_pending_archives(&self, ids: Vec<AgentId>) {
        self.pending_archives.lock().extend(ids);
    }
}

#[cfg(test)]
mod tests {
    use super::CLAUDE_ARCHIVE_GUIDANCE as GUIDANCE;
    use super::*;

    /// Archiving puts the session out of sight, so the guidance is pinned
    /// to explicit asks only, and to finishing the reply before the turn
    /// ends.
    #[test]
    fn the_archive_guidance_is_explicit_ask_only() {
        assert!(GUIDANCE.starts_with("[nebula] Only when the user explicitly asks"));
        assert!(GUIDANCE.contains("never on your own initiative"));
        assert!(GUIDANCE.contains("\n\n  nebula archive\n\n"));
        assert!(GUIDANCE.contains("finish your reply first"));
    }

    #[test]
    fn only_turn_end_signals_end_the_turn() {
        assert!(ends_turn(&HookEvent::Stop));
        assert!(ends_turn(&HookEvent::Progress { busy: false }));
        assert!(ends_turn(&HookEvent::Notification {
            notification_type: Some("idle_prompt".into()),
        }));
        assert!(!ends_turn(&HookEvent::Progress { busy: true }));
        assert!(!ends_turn(&HookEvent::UserPromptSubmit));
        assert!(!ends_turn(&HookEvent::Notification {
            notification_type: Some("permission_prompt".into()),
        }));
    }
}
