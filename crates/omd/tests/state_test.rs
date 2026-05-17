use omd::state::{OmdStateStore, OmdSessionState};
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
