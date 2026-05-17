use crate::fsm::OmdFsm;
use crate::state::{OmdSessionState, OmdStateStore};
use crate::tasks::{Task, TaskGraph, TaskStatus};
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
    pub task_graph: Option<TaskGraph>,
    /// Shell commands executed in current phase (for evidence verification).
    /// Cleared on each successful phase transition.
    pub audit_log: Vec<(String, i32)>,
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
        Self { fsm, session_state, store, task_graph: None, audit_log: Vec::new() }
    }

    /// Create a SharedOmdRuntime (the type tools will hold)
    pub fn shared(agent: OmdAgent, workspace: &Path) -> SharedOmdRuntime {
        Arc::new(RwLock::new(Self::new(agent, workspace)))
    }

    /// Record a shell command execution for evidence verification.
    pub fn push_audit_entry(&mut self, command: String, exit_code: i32) {
        self.audit_log.push((command, exit_code));
    }

    /// Sync task_graph into session_state and persist to disk.
    fn persist_state(&mut self) {
        self.session_state.task_graph = self.task_graph.clone();
        let _ = self.store.write_state(&self.session_state);
    }

    /// Initialize the task graph from a list of tasks. Validates DAG before storing.
    pub fn init_task_graph(&mut self, tasks: Vec<Task>) -> Result<(), String> {
        let mut graph = TaskGraph::new();
        for task in tasks {
            graph.add_task(task);
        }
        graph.validate()?;
        self.task_graph = Some(graph);
        self.persist_state();
        Ok(())
    }

    /// Handle omd_task_update: update status, changed_files, and evidence for a task.
    pub fn handle_task_update(
        &mut self,
        task_id: &str,
        status: TaskStatus,
        changed_files: Vec<String>,
        evidence: Option<Value>,
    ) -> Value {
        if let Some(ref mut graph) = self.task_graph {
            if let Err(e) = graph.set_status(task_id, status) {
                return json!({"ok": false, "error": e});
            }
            if let Some(task) = graph.get_mut(task_id) {
                task.changed_files = changed_files;
                if let Some(ev) = evidence {
                    task.evidence.push(ev);
                }
            }
            let (done, total) = graph.progress();
            let _ = self.store.append_event(
                &self.session_state.session_id,
                &json!({
                    "ts": Utc::now().to_rfc3339(),
                    "event": "task_update",
                    "task_id": task_id,
                    "done": done,
                    "total": total
                }),
            );
            self.persist_state();
            json!({"ok": true, "progress": format!("{}/{}", done, total)})
        } else {
            json!({"ok": false, "error": "No task graph initialized"})
        }
    }

    /// Handle omd_phase_complete. IMPORTANT: capture `from` BEFORE transition.
    pub fn handle_phase_complete(&mut self, next_phase: &str, reason: &str, evidence: &[Value]) -> Value {
        let from = self.fsm.current_phase_name().to_string();

        match self.fsm.try_transition(next_phase) {
            Ok(()) => {
                let to = self.fsm.current_phase_name();
                self.session_state.update_phase(to);
                // Contract 5: append event FIRST (source of truth), then snapshot
                let _ = self.store.write_state_with_event(
                    &self.session_state,
                    &json!({"ts": Utc::now().to_rfc3339(), "event": "phase_transition", "from": from, "to": to, "reason": reason, "evidence": evidence}),
                );
                // Clear audit log on phase transition (evidence is phase-scoped)
                self.audit_log.clear();
                json!({"ok": true, "phase": to, "message": format!("Transitioned from {} to {}. Tool availability updated.", from, to), "tools_changed": true})
            }
            Err(e) => json!({"ok": false, "error": e, "current_phase": from, "valid_next_phases": self.fsm.valid_next_phases()}),
        }
    }

    /// Check if there's an unfinished session at the given workspace.
    /// Used by Hongjun on startup to suggest resumption.
    pub fn detect_unfinished_session(workspace: &Path) -> Option<OmdSessionState> {
        let store = OmdStateStore::new(workspace);
        match store.read_state() {
            Ok(Some(state)) if state.phase != "Done" => Some(state),
            _ => None,
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
        let mut v = json!({
            "agent": format!("{:?}", self.fsm.agent()),
            "phase": self.fsm.current_phase_name(),
            "valid_next_phases": self.fsm.valid_next_phases(),
            "session_id": self.session_state.session_id,
            "started_at": self.session_state.started_at,
        });
        if let Some(ref graph) = self.task_graph {
            let (done, total) = graph.progress();
            v["task_progress"] = json!(format!("{}/{}", done, total));
            v["task_graph"] = serde_json::to_value(graph).unwrap_or_default();
        }
        v
    }
}
