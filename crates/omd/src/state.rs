use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
}
