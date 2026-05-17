use crate::types::*;

/// Evidence types that can satisfy a transition guard requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredEvidence {
    FileDiscovery,
    TestResult,
    GitDiff,
    PlanArtifact,
}

/// Return the required evidence types for a given phase transition.
/// Empty = no specific evidence required (transition allowed unconditionally).
/// ExplicitSkip is always an implicit alternative for any requirement.
pub fn required_evidence_for(current_phase: &OmdPhase, target: &str) -> Vec<RequiredEvidence> {
    match (current_phase, target) {
        // Tongtian
        (OmdPhase::Tongtian(TongtianPhase::Explore), "Execute") => vec![RequiredEvidence::FileDiscovery],
        (OmdPhase::Tongtian(TongtianPhase::Execute), "Verify") => vec![RequiredEvidence::GitDiff],
        (OmdPhase::Tongtian(TongtianPhase::Verify), "Done") => vec![RequiredEvidence::TestResult],
        (OmdPhase::Tongtian(TongtianPhase::Verify), "Execute") => vec![], // loop back

        // Fuxi
        (OmdPhase::Fuxi(FuxiPhase::Explore), "Architect") => vec![RequiredEvidence::FileDiscovery],
        (OmdPhase::Fuxi(FuxiPhase::Plan), "Done") => vec![RequiredEvidence::PlanArtifact],

        // Pangu
        (OmdPhase::Pangu(PanguPhase::Verify), "Done") => vec![RequiredEvidence::TestResult],

        // All other transitions: no specific evidence required
        _ => vec![],
    }
}

/// Check if submitted evidence satisfies requirements. ExplicitSkip always satisfies.
pub fn check_evidence_requirements(
    current_phase: &OmdPhase,
    target: &str,
    evidence_types: &[&str],
) -> Result<(), Vec<RequiredEvidence>> {
    let required = required_evidence_for(current_phase, target);
    if required.is_empty() {
        return Ok(());
    }
    if evidence_types.contains(&"ExplicitSkip") {
        return Ok(());
    }
    let missing: Vec<RequiredEvidence> = required.into_iter()
        .filter(|req| {
            let type_name = match req {
                RequiredEvidence::FileDiscovery => "FileDiscovery",
                RequiredEvidence::TestResult => "TestResult",
                RequiredEvidence::GitDiff => "GitDiff",
                RequiredEvidence::PlanArtifact => "PlanArtifact",
            };
            !evidence_types.contains(&type_name)
        })
        .collect();
    if missing.is_empty() { Ok(()) } else { Err(missing) }
}
