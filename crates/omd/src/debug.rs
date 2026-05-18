//! Debug/monitoring logger for OMD runtime.
//!
//! Writes structured JSONL events to `.omd/debug.jsonl` for production
//! diagnostics. All I/O errors are silently ignored — the debug log must
//! never interfere with the main runtime.

use chrono::Utc;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Lightweight debug logger that appends JSONL events to `.omd/debug.jsonl`.
///
/// Every method is infallible — I/O failures are silently swallowed so the
/// debug subsystem never disrupts the main OMD pipeline.
pub struct OmdDebugLogger {
    path: PathBuf,
}

impl OmdDebugLogger {
    /// Create a new logger rooted at the given workspace directory.
    /// Creates `.omd/` if it doesn't already exist.
    pub fn new(workspace: &Path) -> Self {
        let path = workspace.join(".omd").join("debug.jsonl");
        let _ = fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
        Self { path }
    }

    /// Append a raw JSON event. Injects `"ts"` if absent.
    pub fn log(&self, event: Value) {
        let mut entry = event;
        if entry.get("ts").is_none() {
            entry["ts"] = json!(Utc::now().to_rfc3339());
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(file, "{}", serde_json::to_string(&entry).unwrap_or_default());
        }
    }

    // ── Typed helpers ───────────────────────────────────────────────

    /// Log a tool call (success or blocked).
    pub fn log_tool_call(
        &self,
        agent: &str,
        phase: &str,
        tool: &str,
        status: &str,
        detail: &str,
    ) {
        self.log(json!({
            "event": "tool_call",
            "agent": agent,
            "phase": phase,
            "tool": tool,
            "status": status,
            "detail": detail,
        }));
    }

    /// Log a phase transition (success or failure).
    pub fn log_phase_transition(
        &self,
        agent: &str,
        from: &str,
        to: &str,
        success: bool,
        detail: &str,
    ) {
        self.log(json!({
            "event": if success { "phase_transition" } else { "phase_transition_failed" },
            "agent": agent,
            "from": from,
            "to": to,
            "detail": detail,
        }));
    }

    /// Log evidence skip (unparseable evidence during phase transition).
    pub fn log_evidence_skip(&self, agent: &str, phase: &str, raw: &str) {
        self.log(json!({
            "event": "evidence_skip",
            "agent": agent,
            "phase": phase,
            "raw": raw,
        }));
    }

    /// Log a turn-level event (turn_start / turn_end / stall_hint).
    pub fn log_turn(&self, agent: &str, phase: &str, event_type: &str, detail: Value) {
        self.log(json!({
            "event": event_type,
            "agent": agent,
            "phase": phase,
            "detail": detail,
        }));
    }

    /// Log a mode switch (e.g. OmdHongjun → OmdFuxi).
    pub fn log_mode_switch(&self, from: &str, to: &str) {
        self.log(json!({
            "event": "mode_switch",
            "from": from,
            "to": to,
        }));
    }

    // ── Reader for /omd-debug ───────────────────────────────────────

    /// Read the most recent `limit` entries from the debug log.
    pub fn recent_entries(&self, limit: usize) -> Vec<Value> {
        let content = fs::read_to_string(&self.path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        let start = if lines.len() > limit {
            lines.len() - limit
        } else {
            0
        };
        lines[start..]
            .iter()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    /// Format a human-readable debug summary for `/omd-debug`.
    pub fn format_summary(
        &self,
        agent: &str,
        phase: &str,
        session_id: &str,
        started_at: &str,
        available_tools: &[&str],
    ) -> String {
        let mut out = String::new();
        out.push_str("=== OMD Debug Status ===\n");
        out.push_str(&format!("Agent: {}\n", agent));
        out.push_str(&format!("Phase: {}\n", phase));
        out.push_str(&format!("Session: {}\n", session_id));
        out.push_str(&format!("Started: {}\n", started_at));
        out.push('\n');

        // Available tools
        out.push_str(&format!("Available tools ({}):\n  ", available_tools.len()));
        let tool_line = available_tools.join(", ");
        // Wrap at ~70 chars
        let mut line_len = 2usize;
        let mut first = true;
        for tool in available_tools {
            let sep = if first { "" } else { ", " };
            let addition = sep.len() + tool.len();
            if !first && line_len + addition > 72 {
                out.push_str(",\n  ");
                line_len = 2;
                out.push_str(tool);
                line_len += tool.len();
            } else {
                out.push_str(sep);
                out.push_str(tool);
                line_len += addition;
            }
            first = false;
        }
        if !available_tools.is_empty() {
            out.push('\n');
        }
        let _ = tool_line; // silence unused warning

        // Recent events
        let entries = self.recent_entries(20);
        if !entries.is_empty() {
            out.push_str(&format!("\nRecent events (last {}):\n", entries.len()));
            for entry in &entries {
                let ts = entry
                    .get("ts")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                // Extract just HH:MM:SS from RFC3339
                let time = ts
                    .find('T')
                    .map(|i| &ts[i + 1..])
                    .and_then(|rest| rest.find('+').or(rest.find('Z')).map(|j| &rest[..j]))
                    .unwrap_or(ts);
                let event = entry
                    .get("event")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let detail = match event {
                    "tool_call" => {
                        let tool = entry.get("tool").and_then(|v| v.as_str()).unwrap_or("?");
                        let status = entry.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                        let detail_text = entry.get("detail").and_then(|v| v.as_str()).unwrap_or("");
                        if detail_text.is_empty() {
                            format!("{} -> {}", tool, status.to_uppercase())
                        } else {
                            format!("{} -> {} ({})", tool, status.to_uppercase(), detail_text)
                        }
                    }
                    "phase_transition" | "phase_transition_failed" => {
                        let from = entry.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                        let to = entry.get("to").and_then(|v| v.as_str()).unwrap_or("?");
                        format!("{} -> {}", from, to)
                    }
                    "mode_switch" => {
                        let from = entry.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                        let to = entry.get("to").and_then(|v| v.as_str()).unwrap_or("?");
                        format!("{} -> {}", from, to)
                    }
                    "evidence_skip" => {
                        let raw = entry.get("raw").and_then(|v| v.as_str()).unwrap_or("?");
                        let preview = if raw.len() > 60 { &raw[..60] } else { raw };
                        format!("skipped: {}", preview)
                    }
                    "turn_start" | "turn_end" | "stall_hint" => {
                        let agent_name = entry.get("agent").and_then(|v| v.as_str()).unwrap_or("?");
                        format!("agent={}", agent_name)
                    }
                    _ => {
                        let detail_val = entry.get("detail");
                        match detail_val {
                            Some(Value::String(s)) => s.clone(),
                            Some(v) => serde_json::to_string(v).unwrap_or_default(),
                            None => String::new(),
                        }
                    }
                };
                out.push_str(&format!("  {} [{}] {}\n", time, event, detail));
            }
        }

        // Error summary
        let errors: Vec<&Value> = entries
            .iter()
            .filter(|e| {
                let event = e.get("event").and_then(|v| v.as_str()).unwrap_or("");
                let status = e.get("status").and_then(|v| v.as_str()).unwrap_or("");
                event == "phase_transition_failed"
                    || status == "blocked"
                    || event == "evidence_skip"
            })
            .collect();
        if !errors.is_empty() {
            out.push_str(&format!("\nErrors (last {}):\n", errors.len()));
            // Group blocked tool calls
            let mut blocked_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for e in &errors {
                let event = e.get("event").and_then(|v| v.as_str()).unwrap_or("");
                let status = e.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status == "blocked" {
                    let tool = e.get("tool").and_then(|v| v.as_str()).unwrap_or("?");
                    let phase_name = e.get("phase").and_then(|v| v.as_str()).unwrap_or("?");
                    let key = format!("{} blocked in {} phase", tool, phase_name);
                    *blocked_counts.entry(key).or_insert(0) += 1;
                } else if event == "phase_transition_failed" {
                    let detail = e.get("detail").and_then(|v| v.as_str()).unwrap_or("?");
                    out.push_str(&format!("  phase_transition_failed: {}\n", detail));
                } else if event == "evidence_skip" {
                    out.push_str("  evidence_skip\n");
                }
            }
            for (key, count) in &blocked_counts {
                out.push_str(&format!("  {}x {}\n", count, key));
            }
        }

        // Phase history from entries
        let transitions: Vec<&Value> = entries
            .iter()
            .filter(|e| {
                let event = e.get("event").and_then(|v| v.as_str()).unwrap_or("");
                event == "phase_transition"
            })
            .collect();
        if !transitions.is_empty() {
            out.push_str("\nPhase history:\n");
            for t in &transitions {
                let from = t.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                let to = t.get("to").and_then(|v| v.as_str()).unwrap_or("?");
                let ts = t.get("ts").and_then(|v| v.as_str()).unwrap_or("?");
                let time = ts
                    .find('T')
                    .map(|i| &ts[i + 1..])
                    .and_then(|rest| rest.find('+').or(rest.find('Z')).map(|j| &rest[..j]))
                    .unwrap_or(ts);
                out.push_str(&format!("  {} -> {} ({})\n", from, to, time));
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn log_and_read_round_trip() {
        let tmp = TempDir::new().unwrap();
        let logger = OmdDebugLogger::new(tmp.path());

        logger.log_tool_call("Fuxi", "Explore", "read_file", "ok", "path: src/main.rs");
        logger.log_tool_call("Fuxi", "Explore", "web_search", "blocked", "not in allowlist");
        logger.log_phase_transition("Fuxi", "Interview", "Explore", true, "requirements understood");

        let entries = logger.recent_entries(10);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["event"], "tool_call");
        assert_eq!(entries[0]["tool"], "read_file");
        assert_eq!(entries[0]["status"], "ok");
        assert_eq!(entries[1]["status"], "blocked");
        assert_eq!(entries[2]["event"], "phase_transition");
    }

    #[test]
    fn recent_entries_respects_limit() {
        let tmp = TempDir::new().unwrap();
        let logger = OmdDebugLogger::new(tmp.path());

        for i in 0..10 {
            logger.log_tool_call("Fuxi", "Explore", &format!("tool_{}", i), "ok", "");
        }

        let entries = logger.recent_entries(3);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["tool"], "tool_7");
        assert_eq!(entries[2]["tool"], "tool_9");
    }

    #[test]
    fn format_summary_includes_key_sections() {
        let tmp = TempDir::new().unwrap();
        let logger = OmdDebugLogger::new(tmp.path());

        logger.log_tool_call("Fuxi", "Explore", "web_search", "blocked", "not in allowlist");
        logger.log_tool_call("Fuxi", "Explore", "web_search", "blocked", "not in allowlist");
        logger.log_phase_transition("Fuxi", "Interview", "Explore", true, "");

        let summary = logger.format_summary(
            "Fuxi",
            "Explore",
            "test-session-id",
            "2026-05-18T14:00:00Z",
            &["read_file", "grep_files", "omd_phase_complete"],
        );

        assert!(summary.contains("=== OMD Debug Status ==="));
        assert!(summary.contains("Agent: Fuxi"));
        assert!(summary.contains("Phase: Explore"));
        assert!(summary.contains("Session: test-session-id"));
        assert!(summary.contains("Available tools (3)"));
        assert!(summary.contains("read_file"));
        assert!(summary.contains("Recent events"));
        assert!(summary.contains("[tool_call]"));
        assert!(summary.contains("BLOCKED"));
        assert!(summary.contains("Errors"));
        assert!(summary.contains("web_search blocked in Explore phase"));
        assert!(summary.contains("Phase history"));
    }

    #[test]
    fn log_mode_switch_records_event() {
        let tmp = TempDir::new().unwrap();
        let logger = OmdDebugLogger::new(tmp.path());

        logger.log_mode_switch("OmdHongjun", "OmdFuxi");

        let entries = logger.recent_entries(5);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["event"], "mode_switch");
        assert_eq!(entries[0]["from"], "OmdHongjun");
        assert_eq!(entries[0]["to"], "OmdFuxi");
    }

    #[test]
    fn log_evidence_skip_records_event() {
        let tmp = TempDir::new().unwrap();
        let logger = OmdDebugLogger::new(tmp.path());

        logger.log_evidence_skip("Fuxi", "Interview", r#"{"type":"PlanArtifact","desc":"..."}"#);

        let entries = logger.recent_entries(5);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["event"], "evidence_skip");
        assert_eq!(entries[0]["phase"], "Interview");
    }
}
