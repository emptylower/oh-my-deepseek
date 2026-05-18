use omd::tasks::{TaskGraph, Task, TaskStatus};
use serde_json::json;

#[test]
fn empty_graph() {
    let graph = TaskGraph::new();
    assert_eq!(graph.tasks.len(), 0);
    assert_eq!(graph.next_runnable(), None);
}

#[test]
fn add_task_and_find_runnable() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "First task"));
    graph.add_task(Task::new("T2", "Second task"));
    graph.validate().unwrap();

    let next = graph.next_runnable().unwrap();
    assert_eq!(next, "T1");
}

#[test]
fn dependency_blocks_task() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "First task"));
    let mut t2 = Task::new("T2", "Second task");
    t2.depends_on = vec!["T1".to_string()];
    graph.add_task(t2);
    graph.validate().unwrap();

    // Only T1 is runnable (T2 blocked by T1)
    assert_eq!(graph.next_runnable(), Some("T1".to_string()));

    // Complete T1 (must go Pending → Active → Done)
    graph.set_status("T1", TaskStatus::Active).unwrap();
    graph.set_status("T1", TaskStatus::Done).unwrap();

    // Now T2 is runnable
    assert_eq!(graph.next_runnable(), Some("T2".to_string()));
}

#[test]
fn all_done_returns_true() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "First"));
    graph.validate().unwrap();
    graph.set_status("T1", TaskStatus::Active).unwrap();
    graph.set_status("T1", TaskStatus::Done).unwrap();
    assert!(graph.all_done());
}

#[test]
fn progress_summary() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "A"));
    graph.add_task(Task::new("T2", "B"));
    graph.add_task(Task::new("T3", "C"));
    graph.validate().unwrap();

    graph.set_status("T1", TaskStatus::Active).unwrap();
    graph.set_status("T1", TaskStatus::Done).unwrap();
    graph.set_status("T2", TaskStatus::Active).unwrap();

    let (done, total) = graph.progress();
    assert_eq!(done, 1);
    assert_eq!(total, 3);
}

#[test]
fn dag_validation_rejects_unknown_dep() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "First"));
    let mut t2 = Task::new("T2", "Second");
    t2.depends_on = vec!["NONEXISTENT".to_string()];
    graph.add_task(t2);

    let result = graph.validate();
    assert!(result.is_err(), "Should reject task with dependency on nonexistent task");
    assert!(result.unwrap_err().contains("NONEXISTENT"));
}

#[test]
fn dag_validation_rejects_duplicate_ids() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "First"));
    graph.add_task(Task::new("T1", "Duplicate"));

    let result = graph.validate();
    assert!(result.is_err(), "Should reject duplicate task IDs");
    assert!(result.unwrap_err().contains("Duplicate task ID"));
}

#[test]
fn validate_rejects_cycle() {
    let mut graph = TaskGraph::new();
    let mut t1 = Task::new("T1", "First");
    t1.depends_on = vec!["T2".to_string()];
    let mut t2 = Task::new("T2", "Second");
    t2.depends_on = vec!["T1".to_string()];
    graph.add_task(t1);
    graph.add_task(t2);
    assert!(graph.validate().is_err());
    assert!(graph.validate().unwrap_err().contains("cycle"));
}

#[test]
fn task_has_evidence_and_write_scope() {
    let mut task = Task::new("T1", "Implement feature");
    task.write_scope = vec!["crates/omd/src/**".to_string()];
    task.evidence.push(json!({"type": "test_pass", "output": "15 tests pass"}));

    assert_eq!(task.write_scope.len(), 1);
    assert_eq!(task.evidence.len(), 1);
    assert_eq!(task.evidence[0]["type"], "test_pass");
}

#[test]
fn task_category_field() {
    let mut task = Task::new("T1", "Implement feature");
    task.category = Some("implementation".to_string());

    let json = serde_json::to_value(&task).unwrap();
    assert_eq!(json["category"], "implementation");
}

#[test]
fn valid_status_transitions() {
    use omd::tasks::is_valid_transition;
    assert!(is_valid_transition(&TaskStatus::Pending, &TaskStatus::Active));
    assert!(is_valid_transition(&TaskStatus::Active, &TaskStatus::Done));
    assert!(is_valid_transition(&TaskStatus::Active, &TaskStatus::Failed));
    assert!(is_valid_transition(&TaskStatus::Failed, &TaskStatus::Active));
    assert!(is_valid_transition(&TaskStatus::Pending, &TaskStatus::Skipped));
}

#[test]
fn invalid_status_transitions() {
    use omd::tasks::is_valid_transition;
    assert!(!is_valid_transition(&TaskStatus::Done, &TaskStatus::Active));
    assert!(!is_valid_transition(&TaskStatus::Pending, &TaskStatus::Done));
    assert!(!is_valid_transition(&TaskStatus::Skipped, &TaskStatus::Active));
}

#[test]
fn auto_blocked_and_unblocked() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "First"));
    let mut t2 = Task::new("T2", "Second");
    t2.depends_on = vec!["T1".to_string()];
    graph.add_task(t2);
    graph.validate().unwrap();

    // After validate, T2 is still Pending (recompute not called yet).
    // Call recompute to simulate init_task_graph behavior.
    graph.recompute_blocked_status();

    // T2 should now be auto-blocked (T1 is not done)
    assert_eq!(graph.get("T2").unwrap().status, TaskStatus::Blocked);

    // Only T1 is runnable (T2 is Blocked, not Pending)
    assert_eq!(graph.next_runnable(), Some("T1".to_string()));

    // Complete T1 — recompute is called inside set_status
    graph.set_status("T1", TaskStatus::Active).unwrap();
    graph.set_status("T1", TaskStatus::Done).unwrap();

    // T2 should now be auto-unblocked back to Pending
    assert_eq!(graph.get("T2").unwrap().status, TaskStatus::Pending);
    assert_eq!(graph.next_runnable(), Some("T2".to_string()));
}

#[test]
fn retry_increments_attempts() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "Flaky"));
    graph.validate().unwrap();

    graph.set_status("T1", TaskStatus::Active).unwrap();
    graph.set_status("T1", TaskStatus::Failed).unwrap();
    let t = graph.get("T1").unwrap();
    assert_eq!(t.attempts, 1);

    graph.set_status("T1", TaskStatus::Active).unwrap();
    graph.set_status("T1", TaskStatus::Failed).unwrap();
    let t = graph.get("T1").unwrap();
    assert_eq!(t.attempts, 2);
}

#[test]
fn invalid_transition_returns_error() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "Test"));
    graph.validate().unwrap();

    let result = graph.set_status("T1", TaskStatus::Done);
    assert!(result.is_err());
}

#[test]
fn max_one_active_task() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "First"));
    graph.add_task(Task::new("T2", "Second"));
    graph.validate().unwrap();

    graph.set_status("T1", TaskStatus::Active).unwrap();
    let result = graph.set_status("T2", TaskStatus::Active);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max 1 active task"));
}

#[test]
fn max_attempts_permanently_fails() {
    let mut graph = TaskGraph::new();
    graph.add_task(Task::new("T1", "Flaky"));
    graph.validate().unwrap();

    for _ in 0..3 {
        graph.set_status("T1", TaskStatus::Active).unwrap();
        let _ = graph.set_status("T1", TaskStatus::Failed);
    }
    let result = graph.set_status("T1", TaskStatus::Active);
    assert!(result.is_err());
}
