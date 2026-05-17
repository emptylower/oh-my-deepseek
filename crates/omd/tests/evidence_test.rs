use omd::evidence::{EvidenceClaim, VerificationResult, verify_claim};
use tempfile::tempdir;
use std::fs;

#[test]
fn file_discovery_valid_paths() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("src/main.rs");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "fn main() {}").unwrap();

    let claim = EvidenceClaim::FileDiscovery {
        paths: vec![file.to_string_lossy().to_string()],
    };
    let audit_log: Vec<(String, i32)> = vec![];
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_ok());
}

#[test]
fn file_discovery_nonexistent_rejected() {
    let dir = tempdir().unwrap();
    let claim = EvidenceClaim::FileDiscovery {
        paths: vec!["/nonexistent/path.rs".to_string()],
    };
    let audit_log: Vec<(String, i32)> = vec![];
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_err());
}

#[test]
fn plan_artifact_valid() {
    let dir = tempdir().unwrap();
    let plan = dir.path().join(".omd/plans/test.md");
    fs::create_dir_all(plan.parent().unwrap()).unwrap();
    fs::write(&plan, "# Plan\n- [ ] Task 1\n- [ ] Task 2\n").unwrap();

    let claim = EvidenceClaim::PlanArtifact {
        path: plan.to_string_lossy().to_string(),
    };
    let audit_log: Vec<(String, i32)> = vec![];
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_ok());
}

#[test]
fn plan_artifact_no_checkboxes_rejected() {
    let dir = tempdir().unwrap();
    let plan = dir.path().join(".omd/plans/bad.md");
    fs::create_dir_all(plan.parent().unwrap()).unwrap();
    fs::write(&plan, "# Plan\nNo tasks here\n").unwrap();

    let claim = EvidenceClaim::PlanArtifact {
        path: plan.to_string_lossy().to_string(),
    };
    let audit_log: Vec<(String, i32)> = vec![];
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_err());
}

#[test]
fn explicit_skip_returns_requires_user_ack() {
    let dir = tempdir().unwrap();
    let claim = EvidenceClaim::ExplicitSkip {
        reason: "No tests needed for docs-only change".to_string(),
    };
    let audit_log: Vec<(String, i32)> = vec![];
    let result = verify_claim(&claim, dir.path(), &audit_log).unwrap();
    assert_eq!(result, VerificationResult::RequiresUserAck {
        method: "explicit_skip".to_string(),
        reason: "No tests needed for docs-only change".to_string(),
    });
}

#[test]
fn test_result_matches_audit_log() {
    let dir = tempdir().unwrap();
    let audit_log = vec![
        ("cargo test -p omd".to_string(), 0),
        ("cargo check".to_string(), 0),
    ];
    let claim = EvidenceClaim::TestResult {
        command: "cargo test -p omd".to_string(),
        exit_code: 0,
        stdout_tail: None,
    };
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_ok());
}

#[test]
fn test_result_not_in_audit_log_rejected() {
    let dir = tempdir().unwrap();
    let audit_log = vec![("cargo check".to_string(), 0)];
    let claim = EvidenceClaim::TestResult {
        command: "cargo test -p omd".to_string(),
        exit_code: 0,
        stdout_tail: None,
    };
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_err());
}

#[test]
fn test_result_wrong_exit_code_in_audit_log_rejected() {
    let dir = tempdir().unwrap();
    let audit_log = vec![("cargo test -p omd".to_string(), 1)];
    let claim = EvidenceClaim::TestResult {
        command: "cargo test -p omd".to_string(),
        exit_code: 0,
        stdout_tail: None,
    };
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_err());
}

#[test]
fn git_diff_with_changed_files() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("changed.rs");
    fs::write(&file, "// changed").unwrap();

    let claim = EvidenceClaim::GitDiff {
        changed_files: vec![file.to_string_lossy().to_string()],
    };
    let audit_log: Vec<(String, i32)> = vec![];
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_ok());
}

#[test]
fn git_diff_nonexistent_file_rejected() {
    let dir = tempdir().unwrap();
    let claim = EvidenceClaim::GitDiff {
        changed_files: vec!["/does/not/exist.rs".to_string()],
    };
    let audit_log: Vec<(String, i32)> = vec![];
    assert!(verify_claim(&claim, dir.path(), &audit_log).is_err());
}
