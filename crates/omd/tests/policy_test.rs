use omd::policy::PhaseToolPolicy;
use omd::types::{TongtianPhase, OmdPhase, FuxiPhase, PanguPhase, HongjunPhase};

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
    // exec_shell is allowed in Explore (read-only, per spec)
    assert!(policy.is_allowed("exec_shell"));
    // Delegation (for info-gathering via read-only workers)
    assert!(policy.is_allowed("agent_open"));
    assert!(policy.is_allowed("omd_delegate"));
    // Blocked
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("write_file"));
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

#[test]
fn fuxi_interview_explore_architect_are_read_only() {
    for phase in &[FuxiPhase::Interview, FuxiPhase::Explore, FuxiPhase::Architect] {
        let policy = PhaseToolPolicy::for_phase(&OmdPhase::Fuxi(*phase));
        assert!(policy.is_allowed("read_file"), "Fuxi {:?} must allow read_file", phase);
        assert!(policy.is_allowed("omd_phase_complete"));
        assert!(!policy.is_allowed("edit_file"), "Fuxi {:?} must NOT allow edit_file", phase);
        assert!(!policy.is_allowed("write_file"), "Fuxi {:?} must NOT allow write_file", phase);
    }
    // Interview: no delegation
    let interview = PhaseToolPolicy::for_phase(&OmdPhase::Fuxi(FuxiPhase::Interview));
    assert!(!interview.is_allowed("agent_open"));
    // Explore/Architect: delegation allowed (both omd_delegate and native agent_open)
    let explore = PhaseToolPolicy::for_phase(&OmdPhase::Fuxi(FuxiPhase::Explore));
    assert!(explore.is_allowed("agent_open"));
    assert!(explore.is_allowed("omd_delegate"));
}

#[test]
fn fuxi_plan_phase_allows_omd_write() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Fuxi(FuxiPhase::Plan));
    assert!(policy.is_allowed("write_file"));
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("agent_open"));
}

#[test]
fn pangu_delegate_phase_allows_delegation() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Pangu(PanguPhase::Delegate));
    assert!(policy.is_allowed("omd_delegate"));
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("agent_eval"));
    assert!(policy.is_allowed("agent_close"));
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("write_file"));
}

#[test]
fn pangu_verify_allows_shell_and_delegate_nuwa() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Pangu(PanguPhase::Verify));
    assert!(policy.is_allowed("exec_shell"));
    assert!(policy.is_allowed("omd_delegate"));
    assert!(policy.is_allowed("read_file"));
    assert!(!policy.is_allowed("edit_file"));
}

#[test]
fn hongjun_is_minimal() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Hongjun(HongjunPhase::Intake));
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_state_read"));
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("exec_shell"));
    assert!(!policy.is_allowed("omd_delegate"));
}
