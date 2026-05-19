use omd::workers::WorkerRegistry;

#[test]
fn registry_has_all_workers() {
    let registry = WorkerRegistry::new();
    assert!(registry.get("tongtian-junior").is_some());
    assert!(registry.get("kunpeng").is_some());
    assert!(registry.get("nuwa").is_some());
    assert!(registry.get("shennong").is_some());
    assert!(registry.get("yangmei").is_some());
    assert!(registry.get("cangjie").is_some());
    assert!(registry.get("zhurong").is_some());
    assert!(registry.get("tingfeng").is_some());
}

#[test]
fn unknown_worker_returns_none() {
    let registry = WorkerRegistry::new();
    assert!(registry.get("nonexistent").is_none());
}

#[test]
fn tongtian_junior_can_write_code() {
    let registry = WorkerRegistry::new();
    let w = registry.get("tongtian-junior").unwrap();
    assert!(w.can_write_code);
    assert!(!w.can_delegate);
    assert!(w.allowed_tools.contains(&"edit_file"));
    assert!(w.allowed_tools.contains(&"write_file"));
    assert!(!w.allowed_tools.contains(&"agent_open"));
    assert!(!w.allowed_tools.contains(&"omd_delegate"));
    assert!(!w.allowed_tools.contains(&"omd_checkpoint"));
}

#[test]
fn kunpeng_is_read_only() {
    let registry = WorkerRegistry::new();
    let w = registry.get("kunpeng").unwrap();
    assert!(!w.can_write_code);
    assert!(!w.can_delegate);
    assert!(w.allowed_tools.contains(&"read_file"));
    assert!(w.allowed_tools.contains(&"grep_files"));
    assert!(!w.allowed_tools.contains(&"edit_file"));
    assert!(!w.allowed_tools.contains(&"exec_shell"));
    assert!(!w.allowed_tools.contains(&"omd_checkpoint"));
}

#[test]
fn nuwa_can_run_tests_not_edit() {
    let registry = WorkerRegistry::new();
    let w = registry.get("nuwa").unwrap();
    assert!(!w.can_write_code);
    assert!(w.allowed_tools.contains(&"read_file"));
    assert!(w.allowed_tools.contains(&"exec_shell"));
    assert!(!w.allowed_tools.contains(&"edit_file"));
    // omd_checkpoint NOT in Nuwa's allowed_tools — OMD tools are not registered
    // in the sub-agent tool registry, so including them would fail-close the spawn.
    assert!(!w.allowed_tools.contains(&"omd_checkpoint"));
}

#[test]
fn workers_cannot_delegate() {
    let registry = WorkerRegistry::new();
    for name in &["tongtian-junior", "kunpeng", "nuwa", "shennong", "yangmei", "cangjie", "zhurong", "tingfeng"] {
        let w = registry.get(name).unwrap();
        assert!(!w.can_delegate, "{name} should not be able to delegate");
        assert!(!w.allowed_tools.contains(&"agent_open"), "{name} must not have agent_open");
        assert!(!w.allowed_tools.contains(&"omd_delegate"), "{name} must not have omd_delegate");
    }
}

#[test]
fn no_worker_has_omd_checkpoint() {
    // OMD tools are not registered in the sub-agent tool registry.
    // Including them in allowed_tools would fail-close the spawn.
    let registry = WorkerRegistry::new();
    for name in &["tongtian-junior", "kunpeng", "nuwa", "shennong", "yangmei", "cangjie", "zhurong", "tingfeng"] {
        let w = registry.get(name).unwrap();
        assert!(!w.allowed_tools.contains(&"omd_checkpoint"), "{name} must not have omd_checkpoint");
    }
}

#[test]
fn worker_config_serializes() {
    let registry = WorkerRegistry::new();
    let w = registry.get("kunpeng").unwrap();
    let json = serde_json::to_value(w).unwrap();
    assert_eq!(json["id"], "kunpeng");
    assert_eq!(json["can_write_code"], false);
}
