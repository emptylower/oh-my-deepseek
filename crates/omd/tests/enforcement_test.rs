use omd::{PhaseToolPolicy, WriteScopeValidator};
use omd::shell_policy::{ShellPolicy, validate_command};
use omd::evidence::{EvidenceClaim, VerificationResult, verify_claim};
use omd::types::{OmdPhase, FuxiPhase, PanguPhase, TongtianPhase};
use tempfile::tempdir;
use std::fs;

#[test]
fn fuxi_plan_write_scope_is_omd_only() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Fuxi(FuxiPhase::Plan));
    assert!(policy.is_allowed("write_file"));

    let v = WriteScopeValidator::new(&[".omd/**"]);
    assert!(v.is_allowed(".omd/plans/my-plan.md"));
    assert!(!v.is_allowed("src/main.rs"));
}

#[test]
fn tongtian_explore_has_shell_read_only() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Explore));
    assert!(policy.is_allowed("exec_shell"));
    let shell = policy.shell_policy();
    assert_eq!(shell, ShellPolicy::ReadOnly);
}

#[test]
fn tongtian_verify_has_shell_read_only() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Verify));
    assert!(policy.is_allowed("exec_shell"));
    let shell = policy.shell_policy();
    assert_eq!(shell, ShellPolicy::ReadOnly);
}

#[test]
fn tongtian_execute_has_full_shell() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Tongtian(TongtianPhase::Execute));
    assert!(policy.is_allow_all());
    let shell = policy.shell_policy();
    assert_eq!(shell, ShellPolicy::Full);
}

#[test]
fn pangu_verify_shell_is_read_only() {
    let policy = PhaseToolPolicy::for_phase(&OmdPhase::Pangu(PanguPhase::Verify));
    assert!(policy.is_allowed("exec_shell"));
    let shell = policy.shell_policy();
    assert_eq!(shell, ShellPolicy::ReadOnly);
}

#[test]
fn evidence_file_discovery_end_to_end() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("src/found.rs");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "// found").unwrap();

    let claim = EvidenceClaim::FileDiscovery {
        paths: vec![file.to_string_lossy().to_string()],
    };
    let audit_log: Vec<(String, i32)> = vec![];

    let result = verify_claim(&claim, dir.path(), &audit_log);
    assert!(result.is_ok());
    match result.unwrap() {
        VerificationResult::Verified { method, .. } => {
            assert_eq!(method, "fs_exists");
        }
        _ => panic!("Expected Verified result"),
    }
}

#[test]
fn write_scope_blocks_path_traversal() {
    let v = WriteScopeValidator::new(&["src/**"]);
    assert!(!v.is_allowed("src/../../etc/passwd"));
    assert!(v.is_allowed("src/lib.rs"));
}

#[test]
fn shell_read_blocks_dangerous_commands() {
    assert!(validate_command("rm -rf /", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("git push", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("cargo build", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("echo x > file", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn shell_read_allows_safe_commands() {
    assert!(validate_command("cargo test", ShellPolicy::ReadOnly).is_ok());
    assert!(validate_command("git diff HEAD", ShellPolicy::ReadOnly).is_ok());
    assert!(validate_command("grep -rn pattern src/", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn shell_read_blocks_git_branch_creation() {
    assert!(validate_command("git branch new-branch", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("git tag v1.0", ShellPolicy::ReadOnly).is_err());
    // List forms are allowed
    assert!(validate_command("git branch --list", ShellPolicy::ReadOnly).is_ok());
    assert!(validate_command("git tag --list", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn evidence_audit_log_phase_scoped() {
    let dir = tempdir().unwrap();
    let claim = EvidenceClaim::TestResult {
        command: "cargo test".to_string(),
        exit_code: 0,
        stdout_tail: None,
    };

    // Empty audit log (simulating cleared on phase transition) → fails
    let empty_log: Vec<(String, i32)> = vec![];
    assert!(verify_claim(&claim, dir.path(), &empty_log).is_err());

    // With matching entry → succeeds
    let log = vec![("cargo test".to_string(), 0)];
    assert!(verify_claim(&claim, dir.path(), &log).is_ok());
}

#[test]
fn apply_patch_delete_and_move_validated() {
    let v = WriteScopeValidator::new(&["src/**"]);
    let patch = "*** Delete File: etc/config.yml\n";
    assert!(patch.contains("*** Delete File: "));
    let path = &patch[patch.find("*** Delete File: ").unwrap() + 17..].trim();
    assert!(!v.is_allowed(path));
}
