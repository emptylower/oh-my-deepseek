use omd::PhaseToolPolicy;
use omd::shell_policy::ShellPolicy;
use omd::types::*;

// ── 1. Tongtian::Explore ──────────────────────────────────────────────────────

#[test]
fn tongtian_explore_exact_permissions() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Explore));

    // Not full access
    assert!(!policy.is_allow_all());

    // Shell policy
    assert_eq!(policy.shell_policy(), ShellPolicy::ReadOnly);

    // Allowed tools
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("grep_files"));
    assert!(policy.is_allowed("file_search"));
    assert!(policy.is_allowed("list_dir"));
    assert!(policy.is_allowed("git_status"));
    assert!(policy.is_allowed("git_diff"));
    assert!(policy.is_allowed("git_log"));
    assert!(policy.is_allowed("git_show"));
    assert!(policy.is_allowed("git_blame"));
    assert!(policy.is_allowed("diagnostics"));
    assert!(policy.is_allowed("exec_shell"));
    assert!(policy.is_allowed("exec_shell_wait"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_checkpoint"));
    assert!(policy.is_allowed("omd_state_read"));

    // Blocked tools
    assert!(!policy.is_allowed("write_file"));
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("apply_patch"));
    assert!(!policy.is_allowed("omd_delegate"));
}

// ── 2. Tongtian::Execute ──────────────────────────────────────────────────────

#[test]
fn tongtian_execute_full_access() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Execute));

    // Full access — everything allowed
    assert!(policy.is_allow_all());
    assert_eq!(policy.shell_policy(), ShellPolicy::Full);

    // Spot-check that arbitrary tools are allowed
    assert!(policy.is_allowed("write_file"));
    assert!(policy.is_allowed("edit_file"));
    assert!(policy.is_allowed("exec_shell"));
    assert!(policy.is_allowed("omd_delegate"));
    assert!(policy.is_allowed("read_file"));
}

// ── 3. Tongtian::Verify ───────────────────────────────────────────────────────

#[test]
fn tongtian_verify_exact_permissions() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Verify));

    assert!(!policy.is_allow_all());
    assert_eq!(policy.shell_policy(), ShellPolicy::ReadOnly);

    // Allowed tools
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("grep_files"));
    assert!(policy.is_allowed("file_search"));
    assert!(policy.is_allowed("list_dir"));
    assert!(policy.is_allowed("git_status"));
    assert!(policy.is_allowed("git_diff"));
    assert!(policy.is_allowed("git_log"));
    assert!(policy.is_allowed("git_show"));
    assert!(policy.is_allowed("exec_shell"));
    assert!(policy.is_allowed("exec_shell_wait"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_checkpoint"));
    assert!(policy.is_allowed("omd_state_read"));

    // Blocked tools
    assert!(!policy.is_allowed("write_file"));
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("apply_patch"));
    assert!(!policy.is_allowed("omd_delegate"));
}

// ── 4. Fuxi::Interview ───────────────────────────────────────────────────────

#[test]
fn fuxi_interview_read_only() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Fuxi(FuxiPhase::Interview));

    assert!(!policy.is_allow_all());
    assert_eq!(policy.shell_policy(), ShellPolicy::None);

    // Allowed tools
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("grep_files"));
    assert!(policy.is_allowed("file_search"));
    assert!(policy.is_allowed("list_dir"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_state_read"));

    // Blocked tools
    assert!(!policy.is_allowed("write_file"));
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("exec_shell"));
    assert!(!policy.is_allowed("omd_delegate"));
}

// ── 5. Fuxi::Plan ────────────────────────────────────────────────────────────

#[test]
fn fuxi_plan_limited_write() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Fuxi(FuxiPhase::Plan));

    assert!(!policy.is_allow_all());
    assert_eq!(policy.shell_policy(), ShellPolicy::None);

    // Allowed tools
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("grep_files"));
    assert!(policy.is_allowed("file_search"));
    assert!(policy.is_allowed("list_dir"));
    // write_file is allowed for .omd/plans/ (path validation is separate)
    assert!(policy.is_allowed("write_file"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_state_read"));

    // Blocked tools
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("exec_shell"));
    assert!(!policy.is_allowed("omd_delegate"));
}

// ── 5b. Fuxi::Architect has omd_checkpoint ──────────────────────────────────

#[test]
fn fuxi_architect_has_checkpoint() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Fuxi(FuxiPhase::Architect));
    assert!(policy.is_allowed("omd_checkpoint"), "Fuxi Architect should have omd_checkpoint");
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_state_read"));
    assert!(!policy.is_allowed("write_file"));
}

// ── 5c. Pangu::Decompose has omd_checkpoint ─────────────────────────────────

#[test]
fn pangu_decompose_has_checkpoint() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Pangu(PanguPhase::Decompose));
    assert!(policy.is_allowed("omd_checkpoint"), "Pangu Decompose should have omd_checkpoint");
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_state_read"));
    assert!(!policy.is_allowed("omd_delegate"));
}

// ── 6. Pangu::Delegate ───────────────────────────────────────────────────────

#[test]
fn pangu_delegate_has_delegation() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Pangu(PanguPhase::Delegate));

    assert!(!policy.is_allow_all());
    assert_eq!(policy.shell_policy(), ShellPolicy::None);

    // Allowed tools
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("grep_files"));
    assert!(policy.is_allowed("file_search"));
    assert!(policy.is_allowed("list_dir"));
    assert!(policy.is_allowed("omd_delegate"));
    assert!(policy.is_allowed("agent_eval"));
    assert!(policy.is_allowed("agent_close"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_checkpoint"));
    assert!(policy.is_allowed("omd_state_read"));

    // Blocked tools
    assert!(!policy.is_allowed("write_file"));
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("exec_shell"));
}

// ── 7. Pangu::Verify ─────────────────────────────────────────────────────────

#[test]
fn pangu_verify_has_shell_and_delegation() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Pangu(PanguPhase::Verify));

    assert!(!policy.is_allow_all());
    assert_eq!(policy.shell_policy(), ShellPolicy::ReadOnly);

    // Allowed tools
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("grep_files"));
    assert!(policy.is_allowed("file_search"));
    assert!(policy.is_allowed("list_dir"));
    assert!(policy.is_allowed("exec_shell"));
    assert!(policy.is_allowed("exec_shell_wait"));
    assert!(policy.is_allowed("omd_delegate"));
    assert!(policy.is_allowed("agent_eval"));
    assert!(policy.is_allowed("agent_close"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_checkpoint"));
    assert!(policy.is_allowed("omd_state_read"));

    // Blocked tools
    assert!(!policy.is_allowed("write_file"));
    assert!(!policy.is_allowed("edit_file"));
}

// ── 8. Hongjun::Intake ───────────────────────────────────────────────────────

#[test]
fn hongjun_intake_minimal() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Hongjun(HongjunPhase::Intake));

    assert!(!policy.is_allow_all());
    assert_eq!(policy.shell_policy(), ShellPolicy::None);

    // Allowed tools
    assert!(policy.is_allowed("read_file"));
    assert!(policy.is_allowed("grep_files"));
    assert!(policy.is_allowed("file_search"));
    assert!(policy.is_allowed("list_dir"));
    assert!(policy.is_allowed("omd_phase_complete"));
    assert!(policy.is_allowed("omd_state_read"));

    // Blocked tools
    assert!(!policy.is_allowed("write_file"));
    assert!(!policy.is_allowed("edit_file"));
    assert!(!policy.is_allowed("exec_shell"));
    assert!(!policy.is_allowed("omd_delegate"));
}
