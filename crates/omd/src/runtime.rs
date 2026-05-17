use crate::fsm::OmdFsm;
use crate::state::{OmdSessionState, OmdStateStore};
use crate::types::OmdAgent;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Thread-safe shared OMD runtime — tools hold a clone of this Arc.
pub type SharedOmdRuntime = Arc<RwLock<OmdRuntimeState>>;

/// Mutable runtime state for an active OMD session.
pub struct OmdRuntimeState {
    pub fsm: OmdFsm,
    pub session_state: OmdSessionState,
    pub store: OmdStateStore,
}

impl OmdRuntimeState {
    pub fn new(agent: OmdAgent, workspace: &Path) -> Self {
        let session_id = Uuid::new_v4().to_string();
        let fsm = OmdFsm::new(agent);
        let session_state = OmdSessionState::new(agent, session_id);
        let store = OmdStateStore::new(workspace);
        let _ = store.write_state(&session_state);
        let _ = store.append_event(
            &session_state.session_id,
            &json!({"ts": Utc::now().to_rfc3339(), "event": "session_start", "agent": format!("{:?}", agent), "phase": session_state.phase}),
        );
        Self { fsm, session_state, store }
    }

    /// Create a SharedOmdRuntime (the type tools will hold)
    pub fn shared(agent: OmdAgent, workspace: &Path) -> SharedOmdRuntime {
        Arc::new(RwLock::new(Self::new(agent, workspace)))
    }

    /// Handle omd_phase_complete. IMPORTANT: capture `from` BEFORE transition.
    pub fn handle_phase_complete(&mut self, next_phase: &str, reason: &str, evidence: &[Value]) -> Value {
        let from = self.fsm.current_phase_name().to_string();

        match self.fsm.try_transition(next_phase) {
            Ok(()) => {
                let to = self.fsm.current_phase_name();
                self.session_state.update_phase(to);
                let _ = self.store.write_state(&self.session_state);
                let _ = self.store.append_event(
                    &self.session_state.session_id,
                    &json!({"ts": Utc::now().to_rfc3339(), "event": "phase_transition", "from": from, "to": to, "reason": reason, "evidence": evidence}),
                );
                json!({"ok": true, "phase": to, "message": format!("Transitioned from {} to {}. Tool availability updated.", from, to), "tools_changed": true})
            }
            Err(e) => json!({"ok": false, "error": e, "current_phase": from, "valid_next_phases": self.fsm.valid_next_phases()}),
        }
    }

    /// Handle omd_checkpoint
    pub fn handle_checkpoint(&self, summary: &str) -> Value {
        let _ = self.store.append_event(
            &self.session_state.session_id,
            &json!({"ts": Utc::now().to_rfc3339(), "event": "checkpoint", "phase": self.fsm.current_phase_name(), "summary": summary}),
        );
        json!({"ok": true})
    }

    /// Handle omd_state_read
    pub fn handle_state_read(&self) -> Value {
        json!({
            "agent": format!("{:?}", self.fsm.agent()),
            "phase": self.fsm.current_phase_name(),
            "valid_next_phases": self.fsm.valid_next_phases(),
            "session_id": self.session_state.session_id,
            "started_at": self.session_state.started_at,
        })
    }
}
