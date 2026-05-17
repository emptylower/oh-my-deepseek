use omd::policy::PhaseToolPolicy;
use omd::types::{TongtianPhase, OmdPhase};

#[test]
fn tongtian_explore_allows_read_tools_only() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Explore));
    // Allowed
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("grep_files"));
    assert!(policy.is_allowed("file_search"));
    assert!(policy.is_allowed("list_dir"));
    assert!(policy.is_allowed("git_log"));
    assert!(policy.is_allowed("git_diff"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_state_read"));
    // Blocked
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("write_file"));
    assert!(!policy.is_allowed("exec_shell"));
    assert!(!policy.is_allowed("agent_open"));
}

#[test]
fn tongtian_execute_allows_everything() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Execute));
    assert!(policy.is_allow_all());
    assert!(policy.is_allowed("edit_file"));
    assert!(policy.is_allowed("write_file"));
    assert!(policy.is_allowed("exec_shell"));
    assert!(policy.is_allowed("agent_open"));
}

#[test]
fn tongtian_verify_allows_read_plus_shell() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Verify));
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("exec_shell"));
    assert!(policy.is_allowed("omd_phase_complete"));
    // Blocked
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("write_file"));
}
