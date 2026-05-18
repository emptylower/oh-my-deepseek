#[cfg(unix)]
use libc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::tasks::TaskGraph;
use crate::types::OmdAgent;

/// Current session state — written to current.json on every transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmdSessionState {
    pub schema_version: u32,
    pub agent: String,
    pub phase: String,
    pub started_at: String,
    pub updated_at: String,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_graph: Option<TaskGraph>,
}

impl OmdSessionState {
    pub fn new(agent: OmdAgent, session_id: String) -> Self {
        let now = Utc::now().to_rfc3339();
        let phase = match agent {
            OmdAgent::Tongtian => "Explore",
            OmdAgent::Fuxi => "Interview",
            OmdAgent::Pangu => "LoadPlan",
            OmdAgent::Hongjun => "Intake",
        };
        Self {
            schema_version: 1,
            agent: format!("{:?}", agent),
            phase: phase.to_string(),
            started_at: now.clone(),
            updated_at: now,
            session_id,
            task_graph: None,
        }
    }

    pub fn update_phase(&mut self, phase: &str) {
        self.phase = phase.to_string();
        self.updated_at = Utc::now().to_rfc3339();
    }
}

/// Handles persistence to disk
pub struct OmdStateStore {
    base_dir: PathBuf,
}

impl OmdStateStore {
    pub fn new(workspace: &Path) -> Self {
        let base_dir = workspace.join(".omd").join("sessions");
        Self { base_dir }
    }

    /// Return the workspace root (parent of .omd/sessions/).
    pub fn workspace(&self) -> &Path {
        // base_dir is workspace/.omd/sessions/
        self.base_dir.parent().and_then(|p| p.parent()).unwrap_or(Path::new("."))
    }

    /// Ensure directories exist
    pub fn ensure_dirs(&self, session_id: &str) -> std::io::Result<()> {
        fs::create_dir_all(&self.base_dir)?;
        fs::create_dir_all(self.base_dir.join(session_id))?;
        Ok(())
    }

    /// Write current session state to current.json
    pub fn write_state(&self, state: &OmdSessionState) -> std::io::Result<()> {
        self.ensure_dirs(&state.session_id)?;
        let path = self.base_dir.join("current.json");
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&path, json)?;
        Ok(())
    }

    /// Append an event to the session's events.jsonl
    pub fn append_event(&self, session_id: &str, event: &Value) -> std::io::Result<()> {
        self.ensure_dirs(session_id)?;
        let path = self.base_dir.join(session_id).join("events.jsonl");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let line = serde_json::to_string(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Read current session state from disk
    pub fn read_state(&self) -> std::io::Result<Option<OmdSessionState>> {
        let path = self.base_dir.join("current.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let state: OmdSessionState = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(Some(state))
    }

    /// Clear session state (on session end)
    pub fn clear_state(&self) -> std::io::Result<()> {
        let path = self.base_dir.join("current.json");
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Acquire a session lock. Returns Err if already locked.
    /// Uses OpenOptions::create_new(true) for atomic creation.
    pub fn acquire_lock(&self) -> std::io::Result<()> {
        let lock_path = self.base_dir.join(".lock");
        fs::create_dir_all(&self.base_dir)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "Session lock already held. Another deepseek-omd instance may be running.",
                    )
                } else {
                    e
                }
            })?;
        write!(file, "{}", std::process::id())?;
        Ok(())
    }

    /// Check if lock exists.
    pub fn is_locked(&self) -> bool {
        self.base_dir.join(".lock").exists()
    }

    /// Release the session lock.
    pub fn release_lock(&self) {
        let _ = fs::remove_file(self.base_dir.join(".lock"));
    }

    /// Check for stale lock (dead PID). If stale, remove it.
    /// If alive, return Err. If no lock exists, return Ok.
    pub fn check_stale_lock(&self) -> std::io::Result<()> {
        let lock_path = self.base_dir.join(".lock");
        if !lock_path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&lock_path).unwrap_or_default();
        let pid: u32 = content.trim().parse().unwrap_or(0);
        if pid == 0 {
            let _ = fs::remove_file(&lock_path);
            return Ok(());
        }
        #[cfg(unix)]
        {
            let alive = unsafe { libc::kill(pid as libc::pid_t, 0) == 0 };
            if !alive {
                let _ = fs::remove_file(&lock_path);
                return Ok(());
            }
        }
        #[cfg(not(unix))]
        {
            let _ = fs::remove_file(&lock_path);
            return Ok(());
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Lock held by PID {} (still alive)", pid),
        ))
    }

    /// Release lock only if we own it (PID matches current process).
    pub fn release_lock_owned(&self) {
        let lock_path = self.base_dir.join(".lock");
        if let Ok(content) = fs::read_to_string(&lock_path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if pid == std::process::id() {
                    let _ = fs::remove_file(&lock_path);
                }
            }
        }
    }

    /// Write state with event: append event FIRST (source of truth), then snapshot.
    pub fn write_state_with_event(
        &self,
        state: &OmdSessionState,
        event: &serde_json::Value,
    ) -> std::io::Result<()> {
        self.append_event(&state.session_id, event)?;
        self.write_state(state)?;
        Ok(())
    }

    /// Rebuild phase from events.jsonl (for crash recovery).
    /// Returns the last known phase from phase_transition events.
    pub fn rebuild_from_events(&self, session_id: &str) -> Option<String> {
        let path = self.base_dir.join(session_id).join("events.jsonl");
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(&path).ok()?;
        let mut last_phase = None;
        for line in content.lines() {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
                if event.get("event").and_then(|v| v.as_str()) == Some("phase_transition") {
                    if let Some(to) = event.get("to").and_then(|v| v.as_str()) {
                        last_phase = Some(to.to_string());
                    }
                }
            }
        }
        last_phase
    }

    /// Full state reconstruction from events.jsonl.
    /// Replays all events to reconstruct: agent, phase, and task_graph.
    pub fn rebuild_full_state_from_events(&self, session_id: &str) -> Option<OmdSessionState> {
        let path = self.base_dir.join(session_id).join("events.jsonl");
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(&path).ok()?;

        let mut agent: Option<String> = None;
        let mut phase: Option<String> = None;
        let mut task_graph: Option<TaskGraph> = None;
        let mut started_at: Option<String> = None;

        for line in content.lines() {
            let event: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            match event.get("event").and_then(|v| v.as_str()) {
                Some("session_start") => {
                    agent = event.get("agent").and_then(|v| v.as_str()).map(|s| s.to_string());
                    phase = event.get("phase").and_then(|v| v.as_str()).map(|s| s.to_string());
                    started_at = event.get("ts").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
                Some("phase_transition") => {
                    if let Some(to) = event.get("to").and_then(|v| v.as_str()) {
                        phase = Some(to.to_string());
                    }
                }
                Some("task_update") => {
                    // Rebuild task graph from task_update events.
                    // First appearance of a task carries the full task definition.
                    if let Some(task_json) = event.get("task_definition") {
                        if let Ok(task) = serde_json::from_value::<crate::tasks::Task>(task_json.clone()) {
                            let graph = task_graph.get_or_insert_with(TaskGraph::new);
                            // Only add if task ID is not already in the graph.
                            if graph.get(&task.id).is_none() {
                                graph.add_task(task);
                            }
                        }
                    }
                    // Apply status update.
                    if let Some(task_id) = event.get("task_id").and_then(|v| v.as_str()) {
                        if let Some(status_str) = event.get("status").and_then(|v| v.as_str()) {
                            if let Some(ref mut graph) = task_graph {
                                let status = match status_str {
                                    "Pending" => Some(crate::tasks::TaskStatus::Pending),
                                    "Active" => Some(crate::tasks::TaskStatus::Active),
                                    "Done" => Some(crate::tasks::TaskStatus::Done),
                                    "Failed" => Some(crate::tasks::TaskStatus::Failed),
                                    "Blocked" => Some(crate::tasks::TaskStatus::Blocked),
                                    "Skipped" => Some(crate::tasks::TaskStatus::Skipped),
                                    _ => None,
                                };
                                if let Some(s) = status {
                                    let _ = graph.set_status(task_id, s);
                                }
                            }
                        }
                    }
                }
                _ => {} // checkpoint, fuxi_handoff, etc. don't affect core state
            }
        }

        let agent_str = agent?;
        let phase_str = phase?;

        Some(OmdSessionState {
            schema_version: 1,
            agent: agent_str,
            phase: phase_str,
            started_at: started_at.unwrap_or_default(),
            updated_at: Utc::now().to_rfc3339(),
            session_id: session_id.to_string(),
            task_graph,
        })
    }
}
