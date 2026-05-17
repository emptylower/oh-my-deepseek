use omd::{OmdRuntimeState, OmdAgent, PhaseToolPolicy};
use omd::types::{OmdPhase, TongtianPhase};
use serde_json::json;

#[test]
fn core_hypothesis_tool_blocking_per_phase() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Tongtian, workspace.path());

    // EXPLORE: edit_file blocked
    let policy = PhaseToolPolicy::for_phase(state.fsm.phase());
    assert!(!policy.is_allowed("edit_file"), "edit_file must be blocked in Explore");
    assert!(!policy.is_allowed("write_file"), "write_file must be blocked in Explore");
    assert!(policy.is_allowed("read_file"), "read_file must be allowed in Explore");
    assert!(policy.is_allowed("grep_files"), "grep_files must be allowed in Explore");
    assert!(policy.is_allowed("omd_phase_complete"), "omd_phase_complete must always be allowed");

    // Transition to EXECUTE
    let result = state.handle_phase_complete("Execute", "done exploring", &[json!({"type":"FileDiscovery","paths":["src/main.rs"]})]);
    assert_eq!(result["ok"], true);
    assert_eq!(result["phase"], "Execute");

    // EXECUTE: edit_file allowed
    let policy = PhaseToolPolicy::for_phase(state.fsm.phase());
    assert!(policy.is_allowed("edit_file"), "edit_file must be allowed in Execute");
    assert!(policy.is_allowed("write_file"), "write_file must be allowed in Execute");
    assert!(policy.is_allowed("exec_shell"), "exec_shell must be allowed in Execute");
    assert!(policy.is_allowed("agent_open"), "agent_open must be allowed in Execute");
    assert!(policy.is_allow_all(), "Execute phase must allow all tools");

    // Transition to VERIFY
    let result = state.handle_phase_complete("Verify", "implemented", &[json!({"type":"GitDiff","changed_files":["src/main.rs"]})]);
    assert_eq!(result["ok"], true);
    assert_eq!(result["phase"], "Verify");

    // VERIFY: edit_file blocked again, but exec_shell allowed
    let policy = PhaseToolPolicy::for_phase(state.fsm.phase());
    assert!(!policy.is_allowed("edit_file"), "edit_file must be blocked in Verify");
    assert!(!policy.is_allowed("write_file"), "write_file must be blocked in Verify");
    assert!(policy.is_allowed("exec_shell"), "exec_shell must be allowed in Verify for running tests");
    assert!(policy.is_allowed("read_file"), "read_file must be allowed in Verify");
}

#[test]
fn verify_to_execute_loopback() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Tongtian, workspace.path());

    // Progress to Verify
    state.handle_phase_complete("Execute", "explore done", &[]);
    state.handle_phase_complete("Verify", "impl done", &[]);

    // Loopback to Execute (test failure scenario)
    let result = state.handle_phase_complete("Execute", "tests failed, need to fix", &[json!({"type":"TestFailure","output":"FAILED 2 tests"})]);
    assert_eq!(result["ok"], true);
    assert_eq!(result["phase"], "Execute");

    // Can write again
    let policy = PhaseToolPolicy::for_phase(state.fsm.phase());
    assert!(policy.is_allowed("edit_file"));
}

#[test]
fn invalid_transition_returns_error() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Tongtian, workspace.path());

    // Try to skip from Explore to Done
    let result = state.handle_phase_complete("Done", "skip everything", &[]);
    assert_eq!(result["ok"], false);
    assert!(result["error"].as_str().unwrap().contains("Cannot transition"));
    assert!(result["valid_next_phases"].as_array().unwrap().contains(&json!("Execute")));
}

#[test]
fn state_persists_to_disk() {
    let workspace = tempfile::tempdir().unwrap();
    let state = OmdRuntimeState::new(OmdAgent::Tongtian, workspace.path());

    // Check current.json exists
    let state_file = workspace.path().join(".omd/sessions/current.json");
    assert!(state_file.exists(), "Session state must persist to disk");

    // Check events.jsonl exists
    let events_dir = workspace.path().join(".omd/sessions").join(&state.session_state.session_id);
    let events_file = events_dir.join("events.jsonl");
    assert!(events_file.exists(), "Event log must persist to disk");

    // Verify content
    let state_content = std::fs::read_to_string(&state_file).unwrap();
    let state_json: serde_json::Value = serde_json::from_str(&state_content).unwrap();
    assert_eq!(state_json["agent"], "Tongtian");
    assert_eq!(state_json["phase"], "Explore");
}

#[test]
fn phase_transition_appends_event() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Tongtian, workspace.path());
    let session_id = state.session_state.session_id.clone();

    // Transition
    state.handle_phase_complete("Execute", "done", &[]);

    // Read events
    let events_file = workspace.path().join(".omd/sessions").join(&session_id).join("events.jsonl");
    let content = std::fs::read_to_string(&events_file).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Should have at least 2 events: session_start + phase_transition
    assert!(lines.len() >= 2, "Expected at least 2 events, got {}", lines.len());

    let last_event: serde_json::Value = serde_json::from_str(lines.last().unwrap()).unwrap();
    assert_eq!(last_event["event"], "phase_transition");
    assert_eq!(last_event["from"], "Explore");
    assert_eq!(last_event["to"], "Execute");
}

#[test]
fn checkpoint_saves_without_transition() {
    let workspace = tempfile::tempdir().unwrap();
    let state = OmdRuntimeState::new(OmdAgent::Tongtian, workspace.path());
    let session_id = state.session_state.session_id.clone();

    // Save checkpoint
    let result = state.handle_checkpoint("Found 3 relevant files");
    assert_eq!(result["ok"], true);

    // Verify event was written
    let events_file = workspace.path().join(".omd/sessions").join(&session_id).join("events.jsonl");
    let content = std::fs::read_to_string(&events_file).unwrap();
    assert!(content.contains("checkpoint"));
    assert!(content.contains("Found 3 relevant files"));
}

#[test]
fn state_read_returns_correct_info() {
    let workspace = tempfile::tempdir().unwrap();
    let state = OmdRuntimeState::new(OmdAgent::Tongtian, workspace.path());

    let info = state.handle_state_read();
    assert_eq!(info["agent"], "Tongtian");
    assert_eq!(info["phase"], "Explore");
    assert!(info["valid_next_phases"].as_array().unwrap().contains(&json!("Execute")));
    assert!(info["session_id"].as_str().unwrap().len() > 0);
}
