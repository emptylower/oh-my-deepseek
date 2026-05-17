use omd::transition_guards::{check_evidence_requirements, required_evidence_for, RequiredEvidence};
use omd::types::*;

#[test]
fn tongtian_explore_to_execute_requires_file_discovery() {
    let phase = OmdPhase::Tongtian(TongtianPhase::Explore);
    let required = required_evidence_for(&phase, "Execute");
    assert_eq!(required, vec![RequiredEvidence::FileDiscovery]);
}

#[test]
fn tongtian_execute_to_verify_requires_git_diff() {
    let phase = OmdPhase::Tongtian(TongtianPhase::Execute);
    let required = required_evidence_for(&phase, "Verify");
    assert_eq!(required, vec![RequiredEvidence::GitDiff]);
}

#[test]
fn tongtian_verify_to_done_requires_test_result() {
    let phase = OmdPhase::Tongtian(TongtianPhase::Verify);
    let required = required_evidence_for(&phase, "Done");
    assert_eq!(required, vec![RequiredEvidence::TestResult]);
}

#[test]
fn tongtian_verify_to_execute_requires_nothing() {
    let phase = OmdPhase::Tongtian(TongtianPhase::Verify);
    let required = required_evidence_for(&phase, "Execute");
    assert!(required.is_empty());
}

#[test]
fn fuxi_explore_to_architect_requires_file_discovery() {
    let phase = OmdPhase::Fuxi(FuxiPhase::Explore);
    let required = required_evidence_for(&phase, "Architect");
    assert_eq!(required, vec![RequiredEvidence::FileDiscovery]);
}

#[test]
fn fuxi_plan_to_done_requires_plan_artifact() {
    let phase = OmdPhase::Fuxi(FuxiPhase::Plan);
    let required = required_evidence_for(&phase, "Done");
    assert_eq!(required, vec![RequiredEvidence::PlanArtifact]);
}

#[test]
fn pangu_verify_to_done_requires_test_result() {
    let phase = OmdPhase::Pangu(PanguPhase::Verify);
    let required = required_evidence_for(&phase, "Done");
    assert_eq!(required, vec![RequiredEvidence::TestResult]);
}

#[test]
fn explicit_skip_satisfies_any_requirement() {
    let phase = OmdPhase::Tongtian(TongtianPhase::Explore);
    let result = check_evidence_requirements(&phase, "Execute", &["ExplicitSkip"]);
    assert!(result.is_ok());
}

#[test]
fn matching_evidence_satisfies_requirement() {
    let phase = OmdPhase::Tongtian(TongtianPhase::Explore);
    let result = check_evidence_requirements(&phase, "Execute", &["FileDiscovery"]);
    assert!(result.is_ok());
}

#[test]
fn wrong_evidence_fails() {
    let phase = OmdPhase::Tongtian(TongtianPhase::Explore);
    let result = check_evidence_requirements(&phase, "Execute", &["TestResult"]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), vec![RequiredEvidence::FileDiscovery]);
}

#[test]
fn no_evidence_fails_when_required() {
    let phase = OmdPhase::Tongtian(TongtianPhase::Verify);
    let result = check_evidence_requirements(&phase, "Done", &[]);
    assert!(result.is_err());
}

#[test]
fn unrestricted_transition_always_passes() {
    let phase = OmdPhase::Hongjun(HongjunPhase::Intake);
    let result = check_evidence_requirements(&phase, "Route", &[]);
    assert!(result.is_ok());
}
