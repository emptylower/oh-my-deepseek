use omd::state::{OmdStateStore, OmdSessionState};
use omd::tasks::{Task, TaskGraph};
use omd::types::OmdAgent;
use tempfile::tempdir;
use serde_json::json;

#[test]
fn lock_file_creation_and_release() {
    let dir = tempdir().unwrap();
    let store = OmdStateStore::new(dir.path());

    assert!(store.acquire_lock().is_ok());
    assert!(store.is_locked());
    store.release_lock();
    assert!(!store.is_locked());
}

#[test]
fn lock_prevents_double_acquisition() {
    let dir = tempdir().unwrap();
    let store = OmdStateStore::new(dir.path());

    assert!(store.acquire_lock().is_ok());
    assert!(store.acquire_lock().is_err()); // second lock fails
    store.release_lock();
}

#[test]
fn write_after_append_ordering() {
    let dir = tempdir().unwrap();
    let store = OmdStateStore::new(dir.path());
    let state = OmdSessionState::new(OmdAgent::Pangu, "test-session".to_string());

    store.write_state_with_event(
        &state,
        &json!({"event": "test", "ts": "now"}),
    ).unwrap();

    // Event log should exist
    let events_path = dir.path().join(".omd/sessions/test-session/events.jsonl");
    assert!(events_path.exists());

    // State should exist
    let state_path = dir.path().join(".omd/sessions/current.json");
    assert!(state_path.exists());
}

#[test]
fn rebuild_from_events_restores_phase() {
    let dir = tempdir().unwrap();
    let store = OmdStateStore::new(dir.path());
    let mut state = OmdSessionState::new(OmdAgent::Tongtian, "rebuild-test".to_string());

    store.write_state(&state).unwrap();
    store.append_event("rebuild-test", &json!({
        "event": "phase_transition", "from": "Explore", "to": "Execute"
    })).unwrap();
    state.phase = "Execute".to_string();
    store.write_state(&state).unwrap();

    // Delete current.json to simulate corruption
    std::fs::remove_file(dir.path().join(".omd/sessions/current.json")).unwrap();

    // Rebuild from events
    let rebuilt = store.rebuild_from_events("rebuild-test");
    assert!(rebuilt.is_some());
    assert_eq!(rebuilt.unwrap(), "Execute");
}

// ── Contract 5: rebuild_full_state_from_events ──────────────────────────────

#[test]
fn rebuild_full_state_restores_agent_and_phase() {
    let dir = tempdir().unwrap();
    let store = OmdStateStore::new(dir.path());
    let sid = "full-replay-1";

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

    let state = store.rebuild_full_state_from_events(sid).expect("should rebuild");
    assert_eq!(state.agent, "Tongtian");
    assert_eq!(state.phase, "Execute");
    assert_eq!(state.session_id, sid);
    assert_eq!(state.started_at, "2024-01-01T00:00:00Z");
    assert!(state.task_graph.is_none()); // no task_update events
}

#[test]
fn rebuild_full_state_replays_task_graph() {
    let dir = tempdir().unwrap();
    let store = OmdStateStore::new(dir.path());
    let sid = "full-replay-tasks";

    // Build a sample task definition as JSON
    let task = Task::new("t1", "do the thing");
    let task_json = serde_json::to_value(&task).unwrap();

    store.append_event(sid, &json!({
        "ts": "2024-01-01T00:00:00Z",
        "event": "session_start",
        "agent": "Pangu",
        "phase": "Decompose"
    })).unwrap();
    // First non-Pending status update carries task_definition
    store.append_event(sid, &json!({
        "ts": "2024-01-01T00:01:00Z",
        "event": "task_update",
        "task_id": "t1",
        "status": "Active",
        "done": 0,
        "total": 1,
        "task_definition": task_json
    })).unwrap();
    // Completion event
    store.append_event(sid, &json!({
        "ts": "2024-01-01T00:02:00Z",
        "event": "task_update",
        "task_id": "t1",
        "status": "Done",
        "done": 1,
        "total": 1
    })).unwrap();

    let state = store.rebuild_full_state_from_events(sid).expect("should rebuild");
    assert_eq!(state.phase, "Decompose");
    let graph = state.task_graph.expect("task_graph should be reconstructed");
    let t = graph.get("t1").expect("task t1 should exist");
    assert_eq!(t.description, "do the thing");
    assert_eq!(t.status, omd::tasks::TaskStatus::Done);
}

#[test]
fn rebuild_full_state_returns_none_when_no_events_file() {
    let dir = tempdir().unwrap();
    let store = OmdStateStore::new(dir.path());
    let result = store.rebuild_full_state_from_events("nonexistent-session");
    assert!(result.is_none());
}

#[test]
fn rebuild_full_state_skips_corrupt_lines() {
    let dir = tempdir().unwrap();
    let store = OmdStateStore::new(dir.path());
    let sid = "full-replay-corrupt";

    store.append_event(sid, &json!({
        "ts": "2024-01-01T00:00:00Z",
        "event": "session_start",
        "agent": "Fuxi",
        "phase": "Interview"
    })).unwrap();

    // Inject a corrupt line directly
    let events_path = dir.path().join(".omd/sessions").join(sid).join("events.jsonl");
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().append(true).open(&events_path).unwrap();
    writeln!(f, "{{not valid json}}").unwrap();

    store.append_event(sid, &json!({
        "ts": "2024-01-01T00:01:00Z",
        "event": "phase_transition",
        "from": "Interview",
        "to": "Architect"
    })).unwrap();

    let state = store.rebuild_full_state_from_events(sid).expect("should survive corrupt line");
    assert_eq!(state.phase, "Architect");
}
