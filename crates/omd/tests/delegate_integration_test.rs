use omd::{OmdRuntimeState, OmdAgent, PhaseToolPolicy};
use omd::tasks::{Task, TaskGraph, TaskStatus};
use omd::types::{OmdPhase, PanguPhase, FuxiPhase, HongjunPhase};
use serde_json::json;

#[test]
fn pangu_delegate_phase_allows_omd_delegate() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Pangu, workspace.path());

    // Progress to Delegate phase: LoadPlan → Decompose → Delegate
    state.handle_phase_complete("Decompose", "loaded plan", &[]);
    state.handle_phase_complete("Delegate", "decomposed tasks", &[]);

    let policy = PhaseToolPolicy::for_phase(state.fsm.phase());
    assert!(policy.is_allowed("omd_delegate"));
    assert!(policy.is_allowed("agent_eval"));
    assert!(policy.is_allowed("agent_close"));
    assert!(!policy.is_allowed("edit_file"));
}

#[test]
fn pangu_full_lifecycle() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Pangu, workspace.path());

    assert_eq!(state.fsm.current_phase_name(), "LoadPlan");
    state.handle_phase_complete("Decompose", "plan loaded", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Decompose");
    state.handle_phase_complete("Delegate", "tasks defined", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Delegate");
    state.handle_phase_complete("Verify", "all delegated", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Verify");
    state.handle_phase_complete("Done", "verified", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Done");
}

#[test]
fn fuxi_full_lifecycle() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Fuxi, workspace.path());

    assert_eq!(state.fsm.current_phase_name(), "Interview");
    state.handle_phase_complete("Explore", "questions answered", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Explore");
    state.handle_phase_complete("Architect", "explored", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Architect");
    state.handle_phase_complete("Plan", "designed", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Plan");
    state.handle_phase_complete("Done", "plan written", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Done");
}

#[test]
fn fuxi_plan_phase_is_only_writable_phase() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Fuxi, workspace.path());

    // Interview — no write
    let policy = PhaseToolPolicy::for_phase(state.fsm.phase());
    assert!(!policy.is_allowed("write_file"));

    // Advance to Plan
    state.handle_phase_complete("Explore", "done", &[]);
    state.handle_phase_complete("Architect", "done", &[]);
    state.handle_phase_complete("Plan", "done", &[]);

    // Plan — can write
    let policy = PhaseToolPolicy::for_phase(state.fsm.phase());
    assert!(policy.is_allowed("write_file"));
}

#[test]
fn hongjun_short_lifecycle() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Hongjun, workspace.path());

    assert_eq!(state.fsm.current_phase_name(), "Intake");
    state.handle_phase_complete("Route", "classified", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Route");
    state.handle_phase_complete("Done", "routed to Fuxi", &[]);
    assert_eq!(state.fsm.current_phase_name(), "Done");
}

#[test]
fn session_resumption_detects_unfinished() {
    let workspace = tempfile::tempdir().unwrap();
    let _state = OmdRuntimeState::new(OmdAgent::Pangu, workspace.path());

    // Should detect unfinished session (phase is LoadPlan, not Done)
    let detected = OmdRuntimeState::detect_unfinished_session(workspace.path());
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().phase, "LoadPlan");
}

#[test]
fn session_resumption_ignores_completed() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Hongjun, workspace.path());
    state.handle_phase_complete("Route", "done", &[]);
    state.handle_phase_complete("Done", "routed", &[]);

    // Phase is "Done" → should NOT detect as unfinished
    let detected = OmdRuntimeState::detect_unfinished_session(workspace.path());
    assert!(detected.is_none());
}

#[test]
fn task_graph_dag_validation_in_runtime() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Pangu, workspace.path());

    let mut t1 = Task::new("T1", "First");
    t1.category = Some("implementation".to_string());
    t1.write_scope = vec!["crates/omd/src/**".to_string()];

    let mut t2 = Task::new("T2", "Second");
    t2.depends_on = vec!["T1".to_string()];
    t2.category = Some("test".to_string());

    state.init_task_graph(vec![t1, t2]).unwrap();

    let graph = state.task_graph.as_ref().unwrap();
    assert_eq!(graph.tasks.len(), 2);
    assert_eq!(graph.next_runnable(), Some("T1".to_string()));
}

#[test]
fn task_update_with_evidence() {
    let workspace = tempfile::tempdir().unwrap();
    let mut state = OmdRuntimeState::new(OmdAgent::Pangu, workspace.path());

    let t1 = Task::new("T1", "Implement feature");
    state.init_task_graph(vec![t1]).unwrap();

    let evidence = json!({"type": "test_pass", "output": "15 tests pass", "exit_code": 0});
    let result = state.handle_task_update(
        "T1",
        TaskStatus::Done,
        vec!["crates/omd/src/workers.rs".to_string()],
        Some(evidence.clone()),
    );

    assert_eq!(result["ok"], true);
    assert_eq!(result["progress"], "1/1");

    let task = state.task_graph.as_ref().unwrap().get("T1").unwrap();
    assert_eq!(task.evidence.len(), 1);
    assert_eq!(task.evidence[0]["type"], "test_pass");
    assert_eq!(task.changed_files, vec!["crates/omd/src/workers.rs"]);
}
