use crate::fsm::OmdFsm;
use crate::state::{OmdSessionState, OmdStateStore};
use crate::tasks::{Task, TaskGraph, TaskStatus};
use crate::types::OmdAgent;
use chrono::Utc;
use serde_json::{json, Value};
use std::cmp::Reverse;
use std::fs;
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
    pub fn new(agent: OmdAgent, workspace: &Path) -> Result<Self, String> {
        let session_id = Uuid::new_v4().to_string();
        let fsm = OmdFsm::new(agent);
        let session_state = OmdSessionState::new(agent, session_id);
        let store = OmdStateStore::new(workspace);
        // Fail-closed: check stale lock + acquire
        store.check_stale_lock().map_err(|e| e.to_string())?;
        store.acquire_lock().map_err(|e| format!("Cannot acquire OMD session lock: {}", e))?;
        let _ = store.write_state(&session_state);
        let _ = store.append_event(
            &session_state.session_id,
            &json!({"ts": Utc::now().to_rfc3339(), "event": "session_start", "agent": format!("{:?}", agent), "phase": session_state.phase}),
        );
        Ok(Self { fsm, session_state, store, task_graph: None, audit_log: Vec::new() })
    }

    /// Create a SharedOmdRuntime (the type tools will hold)
    pub fn shared(agent: OmdAgent, workspace: &Path) -> Result<SharedOmdRuntime, String> {
        Ok(Arc::new(RwLock::new(Self::new(agent, workspace)?)))
    }

    /// Resume an existing session from persisted state.
    /// Acquires lock, hydrates FSM to the persisted phase, restores task_graph.
    pub fn resume(workspace: &Path, session_state: OmdSessionState) -> Result<Self, String> {
        let store = OmdStateStore::new(workspace);
        store.check_stale_lock().map_err(|e| e.to_string())?;
        store.acquire_lock().map_err(|e| format!("Cannot acquire lock: {}", e))?;

        // Full state from events — gives us the authoritative phase and task_graph.
        let event_state = store.rebuild_full_state_from_events(&session_state.session_id);

        // Determine the correct phase: prefer events.jsonl over current.json.
        let effective_phase = event_state
            .as_ref()
            .map(|s| s.phase.clone())
            .or_else(|| store.rebuild_from_events(&session_state.session_id))
            .unwrap_or_else(|| session_state.phase.clone());

        if effective_phase != session_state.phase {
            tracing::warn!(
                session_id = %session_state.session_id,
                current_json_phase = %session_state.phase,
                event_phase = %effective_phase,
                "Phase mismatch between current.json and events.jsonl — using event-derived phase"
            );
        }

        // Determine the agent from the session state
        let agent = match session_state.agent.as_str() {
            "Tongtian" => OmdAgent::Tongtian,
            "Fuxi" => OmdAgent::Fuxi,
            "Pangu" => OmdAgent::Pangu,
            "Hongjun" => OmdAgent::Hongjun,
            other => return Err(format!("Unknown agent: {}", other)),
        };

        // Hydrate FSM at the recovered phase
        let fsm = OmdFsm::with_phase(agent, &effective_phase)?;

        // Restore task_graph: prefer current.json (most complete), fall back to events replay.
        let task_graph = session_state.task_graph.clone().or_else(|| {
            event_state.as_ref().and_then(|s| s.task_graph.clone())
        });

        // Update current.json if phase or task_graph was corrected
        let mut corrected_state = session_state;
        let phase_changed = corrected_state.phase != effective_phase;
        let graph_restored = corrected_state.task_graph.is_none() && task_graph.is_some();
        if phase_changed {
            corrected_state.phase = effective_phase;
        }
        if graph_restored {
            corrected_state.task_graph = task_graph.clone();
        }
        if phase_changed || graph_restored {
            let _ = store.write_state(&corrected_state);
        }

        Ok(Self {
            fsm,
            session_state: corrected_state,
            store,
            task_graph,
            audit_log: Vec::new(),
        })
    }

    /// Create a SharedOmdRuntime from a resumed session
    pub fn shared_resume(workspace: &Path, session_state: OmdSessionState) -> Result<SharedOmdRuntime, String> {
        Ok(Arc::new(RwLock::new(Self::resume(workspace, session_state)?)))
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
        graph.recompute_blocked_status(); // Initial blocked assignment
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
            let status_str = format!("{:?}", status);

            // Capture task definition before mutating (for event replay).
            // Include on first non-Pending status change so replay can reconstruct tasks.
            let task_definition = if !matches!(status, TaskStatus::Pending) {
                graph.get(task_id).and_then(|t| serde_json::to_value(t).ok())
            } else {
                None
            };

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

            let mut event_json = json!({
                "ts": Utc::now().to_rfc3339(),
                "event": "task_update",
                "task_id": task_id,
                "status": status_str,
                "done": done,
                "total": total
            });
            // Include full task definition for event replay (crash recovery).
            if let Some(def) = task_definition {
                event_json["task_definition"] = def;
            }

            let _ = self.store.append_event(
                &self.session_state.session_id,
                &event_json,
            );

            // Check for permanent failure routing BEFORE dropping the graph borrow
            let routing = if status_str == "Failed" {
                graph.get(task_id).and_then(|task| {
                    if task.attempts >= task.max_attempts {
                        Some((task.max_attempts, task_id.to_string()))
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            self.persist_state();

            let mut result = json!({"ok": true, "progress": format!("{}/{}", done, total)});
            if let Some((max_attempts, tid)) = routing {
                result["routing_suggestion"] = json!("zhurong");
                result["routing_reason"] = json!(format!(
                    "Task '{}' permanently failed after {} attempts. Consider delegating to Zhurong for debugging.",
                    tid, max_attempts
                ));
            }
            result
        } else {
            json!({"ok": false, "error": "No task graph initialized"})
        }
    }

    /// Handle omd_phase_complete. IMPORTANT: capture `from` BEFORE transition.
    pub fn handle_phase_complete(&mut self, next_phase: &str, reason: &str, evidence: &[Value]) -> Value {
        let from = self.fsm.current_phase_name().to_string();

        // Structural guard: Delegate→Verify requires all delegated tasks returned
        if self.fsm.current_phase_name() == "Delegate" && next_phase == "Verify" {
            if let Some(ref graph) = self.task_graph {
                let active_count = graph.tasks().iter()
                    .filter(|t| matches!(t.status, crate::tasks::TaskStatus::Active))
                    .count();
                if active_count > 0 {
                    return json!({
                        "ok": false,
                        "error": format!("Cannot enter Verify: {} task(s) still Active. All delegated tasks must return before verification.", active_count),
                        "current_phase": self.fsm.current_phase_name(),
                    });
                }
            }
        }

        match self.fsm.try_transition(next_phase) {
            Ok(()) => {
                let to = self.fsm.current_phase_name();
                self.session_state.update_phase(to);
                // Contract 5: append event FIRST (source of truth), then snapshot
                let _ = self.store.write_state_with_event(
                    &self.session_state,
                    &json!({"ts": Utc::now().to_rfc3339(), "event": "phase_transition", "from": from, "to": to, "reason": reason, "evidence": evidence}),
                );
                // Fuxi handoff event: emit when Fuxi finishes planning (Plan→Done)
                let fuxi_handoff_plan_path = if matches!(self.fsm.agent(), OmdAgent::Fuxi) && to == "Done" {
                    let plan_path = evidence.iter()
                        .filter_map(|e| e.get("path").and_then(|p| p.as_str()))
                        .next()
                        .unwrap_or(".omd/plans/latest.md");
                    let _ = self.store.append_event(
                        &self.session_state.session_id,
                        &json!({
                            "ts": Utc::now().to_rfc3339(),
                            "event": "fuxi_handoff",
                            "plan_path": plan_path,
                            "message": "Plan ready. Use /omd-execute to start Pangu, or Tab to choose agent.",
                        }),
                    );
                    Some(plan_path.to_string())
                } else {
                    None
                };
                // Clear audit log on phase transition (evidence is phase-scoped)
                self.audit_log.clear();
                let mut result = json!({"ok": true, "phase": to, "message": format!("Transitioned from {} to {}. Tool availability updated.", from, to), "tools_changed": true});
                if let Some(plan_path) = fuxi_handoff_plan_path {
                    result["fuxi_handoff"] = json!(true);
                    result["plan_path"] = json!(plan_path);
                }
                result
            }
            Err(e) => json!({"ok": false, "error": e, "current_phase": from, "valid_next_phases": self.fsm.valid_next_phases()}),
        }
    }

    /// User-initiated phase complete (via /omd-phase-complete).
    /// Bypasses evidence verification but enforces FSM + structural guards.
    pub fn handle_user_phase_complete(&mut self, next_phase: &str) -> Value {
        let from = self.fsm.current_phase_name().to_string();

        // Structural guard: Delegate→Verify requires all delegated tasks returned
        if self.fsm.current_phase_name() == "Delegate" && next_phase == "Verify" {
            if let Some(ref graph) = self.task_graph {
                let active_count = graph.tasks().iter()
                    .filter(|t| matches!(t.status, crate::tasks::TaskStatus::Active))
                    .count();
                if active_count > 0 {
                    return json!({
                        "ok": false,
                        "error": format!("Cannot enter Verify: {} task(s) still Active. All delegated tasks must return before verification.", active_count),
                        "current_phase": self.fsm.current_phase_name(),
                    });
                }
            }
        }

        // 1. FSM validity (same as handle_phase_complete)
        match self.fsm.try_transition(next_phase) {
            Ok(()) => {
                let to = self.fsm.current_phase_name();
                self.session_state.update_phase(to);

                // Insert ExplicitSkip evidence automatically
                let evidence = vec![json!({
                    "type": "ExplicitSkip",
                    "reason": "User-initiated via /omd-phase-complete"
                })];

                let _ = self.store.write_state_with_event(
                    &self.session_state,
                    &json!({
                        "ts": Utc::now().to_rfc3339(),
                        "event": "phase_transition",
                        "from": from,
                        "to": to,
                        "reason": "User-initiated phase complete",
                        "evidence": evidence,
                        "user_initiated": true
                    }),
                );

                // Fuxi handoff (same as normal)
                if matches!(self.fsm.agent(), OmdAgent::Fuxi) && to == "Done" {
                    let _ = self.store.append_event(
                        &self.session_state.session_id,
                        &json!({
                            "ts": Utc::now().to_rfc3339(),
                            "event": "fuxi_handoff",
                            "plan_path": ".omd/plans/latest.md",
                            "message": "Plan ready. Use /omd-execute to start Pangu.",
                        }),
                    );
                }

                self.audit_log.clear();
                json!({
                    "ok": true,
                    "phase": to,
                    "message": format!("User forced transition from {} to {}. Tool availability updated.", from, to),
                    "tools_changed": true,
                    "user_initiated": true
                })
            }
            Err(e) => json!({
                "ok": false,
                "error": e,
                "current_phase": from,
                "valid_next_phases": self.fsm.valid_next_phases()
            }),
        }
    }

    /// Check if there's an unfinished session at the given workspace.
    /// Used by Hongjun on startup to suggest resumption.
    /// Falls back to scanning session directories when current.json is missing or corrupt.
    pub fn detect_unfinished_session(workspace: &Path) -> Option<OmdSessionState> {
        let store = OmdStateStore::new(workspace);

        // Primary: try current.json
        match store.read_state() {
            Ok(Some(state)) if state.phase != "Done" => return Some(state),
            Ok(Some(_)) => return None, // phase == Done, no resume needed
            _ => {}                     // missing or corrupt — fall through to scan
        }

        // Fallback: scan session directories for one with events.jsonl
        let sessions_dir = workspace.join(".omd").join("sessions");
        if !sessions_dir.exists() {
            return None;
        }

        // Find most recent session directory (by modification time)
        let mut entries: Vec<_> = fs::read_dir(&sessions_dir)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && e.path().join("events.jsonl").exists())
            .collect();
        entries.sort_by_key(|e| {
            Reverse(e.metadata().ok().and_then(|m| m.modified().ok()))
        });

        if let Some(entry) = entries.first() {
            let session_id = entry.file_name().to_string_lossy().to_string();
            if let Some(state) = store.rebuild_full_state_from_events(&session_id) {
                if state.phase != "Done" {
                    return Some(state);
                }
            }
        }

        None
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

impl Drop for OmdRuntimeState {
    fn drop(&mut self) {
        self.store.release_lock_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_runtime(agent: OmdAgent) -> (OmdRuntimeState, TempDir) {
        let tmp = TempDir::new().unwrap();
        let rt = OmdRuntimeState::new(agent, tmp.path()).unwrap();
        (rt, tmp)
    }

    #[test]
    fn user_phase_complete_transitions_on_valid_target() {
        let (mut rt, _tmp) = make_runtime(OmdAgent::Tongtian);
        assert_eq!(rt.fsm.current_phase_name(), "Explore");

        let result = rt.handle_user_phase_complete("Execute");
        assert_eq!(result.get("ok"), Some(&serde_json::json!(true)));
        assert_eq!(result.get("phase"), Some(&serde_json::json!("Execute")));
        assert_eq!(result.get("user_initiated"), Some(&serde_json::json!(true)));
        assert_eq!(result.get("tools_changed"), Some(&serde_json::json!(true)));
        assert_eq!(rt.fsm.current_phase_name(), "Execute");
    }

    #[test]
    fn user_phase_complete_rejects_invalid_target() {
        let (mut rt, _tmp) = make_runtime(OmdAgent::Tongtian);
        assert_eq!(rt.fsm.current_phase_name(), "Explore");

        // "Done" is not a valid successor of "Explore" for Tongtian
        let result = rt.handle_user_phase_complete("Done");
        assert_eq!(result.get("ok"), Some(&serde_json::json!(false)));
        assert!(result.get("error").is_some());
        // Phase should remain unchanged
        assert_eq!(rt.fsm.current_phase_name(), "Explore");
    }

    #[test]
    fn user_phase_complete_rejects_nonexistent_phase() {
        let (mut rt, _tmp) = make_runtime(OmdAgent::Fuxi);
        assert_eq!(rt.fsm.current_phase_name(), "Interview");

        let result = rt.handle_user_phase_complete("NonexistentPhase");
        assert_eq!(result.get("ok"), Some(&serde_json::json!(false)));
        assert!(result.get("error").unwrap().as_str().unwrap().contains("not a valid phase"));
        assert_eq!(rt.fsm.current_phase_name(), "Interview");
    }

    #[test]
    fn user_phase_complete_clears_audit_log() {
        let (mut rt, _tmp) = make_runtime(OmdAgent::Tongtian);
        rt.push_audit_entry("cargo test".to_string(), 0);
        assert_eq!(rt.audit_log.len(), 1);

        let result = rt.handle_user_phase_complete("Execute");
        assert_eq!(result.get("ok"), Some(&serde_json::json!(true)));
        assert!(rt.audit_log.is_empty());
    }

    #[test]
    fn delegate_to_verify_blocked_while_active_task_exists() {
        let (mut rt, _tmp) = make_runtime(OmdAgent::Pangu);
        // Walk Pangu to Delegate phase
        rt.handle_user_phase_complete("Decompose");
        rt.handle_user_phase_complete("Delegate");
        assert_eq!(rt.fsm.current_phase_name(), "Delegate");

        // Add a task graph with one Active task
        let mut task = crate::tasks::Task::new("t1", "do work");
        task.status = crate::tasks::TaskStatus::Active;
        rt.task_graph = Some({
            let mut g = crate::tasks::TaskGraph::new();
            g.add_task(task);
            g
        });

        // Attempting Delegate→Verify must fail
        let result = rt.handle_user_phase_complete("Verify");
        assert_eq!(result.get("ok"), Some(&serde_json::json!(false)));
        let err = result.get("error").and_then(|e| e.as_str()).unwrap_or("");
        assert!(err.contains("Cannot enter Verify"), "unexpected error: {}", err);
        assert!(err.contains("1 task(s) still Active"), "unexpected error: {}", err);
        // Phase must remain Delegate
        assert_eq!(rt.fsm.current_phase_name(), "Delegate");
    }

    #[test]
    fn delegate_to_verify_allowed_when_no_active_tasks() {
        let (mut rt, _tmp) = make_runtime(OmdAgent::Pangu);
        // Walk Pangu to Delegate phase
        rt.handle_user_phase_complete("Decompose");
        rt.handle_user_phase_complete("Delegate");
        assert_eq!(rt.fsm.current_phase_name(), "Delegate");

        // Add a task graph where all tasks are Done/Pending (no Active)
        let task = crate::tasks::Task::new("t1", "do work"); // defaults to Pending
        rt.task_graph = Some({
            let mut g = crate::tasks::TaskGraph::new();
            g.add_task(task);
            g
        });

        let result = rt.handle_user_phase_complete("Verify");
        assert_eq!(result.get("ok"), Some(&serde_json::json!(true)));
        assert_eq!(rt.fsm.current_phase_name(), "Verify");
    }

    #[test]
    fn handle_phase_complete_delegate_to_verify_blocked_while_active_task_exists() {
        let (mut rt, _tmp) = make_runtime(OmdAgent::Pangu);
        // Walk Pangu to Delegate phase
        rt.handle_user_phase_complete("Decompose");
        rt.handle_user_phase_complete("Delegate");
        assert_eq!(rt.fsm.current_phase_name(), "Delegate");

        // Add a task graph with one Active task
        let mut task = crate::tasks::Task::new("t2", "model work");
        task.status = crate::tasks::TaskStatus::Active;
        rt.task_graph = Some({
            let mut g = crate::tasks::TaskGraph::new();
            g.add_task(task);
            g
        });

        // Model-facing path: handle_phase_complete must also reject Delegate→Verify with active tasks
        let result = rt.handle_phase_complete("Verify", "all done", &[]);
        assert_eq!(result.get("ok"), Some(&serde_json::json!(false)));
        let err = result.get("error").and_then(|e| e.as_str()).unwrap_or("");
        assert!(err.contains("Cannot enter Verify"), "unexpected error: {}", err);
        assert!(err.contains("1 task(s) still Active"), "unexpected error: {}", err);
        // Phase must remain Delegate
        assert_eq!(rt.fsm.current_phase_name(), "Delegate");
    }

    #[test]
    fn user_phase_complete_fuxi_handoff_on_done() {
        let (mut rt, _tmp) = make_runtime(OmdAgent::Fuxi);
        // Walk through Fuxi phases to get to Plan
        rt.handle_user_phase_complete("Explore");
        rt.handle_user_phase_complete("Architect");
        rt.handle_user_phase_complete("Plan");
        assert_eq!(rt.fsm.current_phase_name(), "Plan");

        // Now transition to Done — should emit fuxi_handoff
        let result = rt.handle_user_phase_complete("Done");
        assert_eq!(result.get("ok"), Some(&serde_json::json!(true)));
        assert_eq!(result.get("phase"), Some(&serde_json::json!("Done")));
    }

    // ── Contract 5: crash recovery tests ────────────────────────────────────

    #[test]
    fn detect_unfinished_session_falls_back_to_events_when_current_json_missing() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path();
        let store = OmdStateStore::new(workspace);
        let sid = "crash-session";

        // Write session_start and a phase_transition event (no current.json)
        store.append_event(sid, &json!({
            "ts": "2024-01-01T00:00:00Z",
            "event": "session_start",
            "agent": "Tongtian",
            "phase": "Explore"
        })).unwrap();
        store.append_event(sid, &json!({
            "ts": "2024-01-01T00:01:00Z",
            "event": "phase_transition",
            "from": "Explore",
            "to": "Execute"
        })).unwrap();

        // No current.json written — detect should fall back to events scan
        let detected = OmdRuntimeState::detect_unfinished_session(workspace);
        assert!(detected.is_some(), "should detect session from events.jsonl");
        let state = detected.unwrap();
        assert_eq!(state.phase, "Execute");
        assert_eq!(state.agent, "Tongtian");
    }

    #[test]
    fn detect_unfinished_session_returns_none_when_phase_is_done() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path();
        let store = OmdStateStore::new(workspace);
        let sid = "done-session";

        store.append_event(sid, &json!({
            "ts": "2024-01-01T00:00:00Z",
            "event": "session_start",
            "agent": "Tongtian",
            "phase": "Explore"
        })).unwrap();
        store.append_event(sid, &json!({
            "ts": "2024-01-01T00:01:00Z",
            "event": "phase_transition",
            "from": "Explore",
            "to": "Done"
        })).unwrap();

        let detected = OmdRuntimeState::detect_unfinished_session(workspace);
        assert!(detected.is_none(), "Done sessions should not be detected as unfinished");
    }

    #[test]
    fn task_update_includes_task_definition_on_first_nontrivial_status() {
        let (mut rt, tmp) = make_runtime(OmdAgent::Pangu);
        let sid = rt.session_state.session_id.clone();

        // Initialize a task graph
        let task = crate::tasks::Task::new("impl-1", "implement feature");
        rt.init_task_graph(vec![task]).unwrap();

        // Transition task to Active (first non-Pending status)
        let result = rt.handle_task_update(
            "impl-1",
            TaskStatus::Active,
            vec![],
            None,
        );
        assert_eq!(result.get("ok"), Some(&serde_json::json!(true)));

        // Verify the emitted event contains task_definition
        let events_path = tmp.path()
            .join(".omd/sessions")
            .join(&sid)
            .join("events.jsonl");
        let content = std::fs::read_to_string(&events_path).unwrap();
        let task_update_event = content
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("task_update"))
            .expect("task_update event should exist");

        assert!(
            task_update_event.get("task_definition").is_some(),
            "task_update event should include task_definition on first status change"
        );
        assert_eq!(
            task_update_event["task_definition"]["id"].as_str(),
            Some("impl-1")
        );
    }

    #[test]
    fn resume_restores_task_graph_from_events_when_current_json_missing_graph() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path();

        // Simulate a session: create runtime, add tasks, then wipe task_graph from current.json
        {
            let mut rt = OmdRuntimeState::new(OmdAgent::Pangu, workspace).unwrap();
            let _sid = rt.session_state.session_id.clone();
            let task = crate::tasks::Task::new("t-resume", "resumable task");
            rt.init_task_graph(vec![task]).unwrap();

            // Mark it Active (emits task_definition in event)
            rt.handle_task_update("t-resume", TaskStatus::Active, vec![], None);

            // Simulate corrupt current.json by stripping task_graph from it
            let store = OmdStateStore::new(workspace);
            let mut state = store.read_state().unwrap().unwrap();
            state.task_graph = None;
            store.write_state(&state).unwrap();
        }

        // Now resume — should restore task_graph from events
        let store = OmdStateStore::new(workspace);
        let state_from_disk = store.read_state().unwrap().unwrap();
        assert!(state_from_disk.task_graph.is_none(), "task_graph was wiped");

        let resumed = OmdRuntimeState::resume(workspace, state_from_disk).unwrap();
        let graph = resumed.task_graph.clone().expect("resume should restore task_graph from events");
        assert!(graph.get("t-resume").is_some(), "task should be in restored graph");
    }
}
