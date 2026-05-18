//! Native OMD tools implementing ToolSpec.
//! These wrap omd::OmdRuntimeState via SharedOmdRuntime.

use async_trait::async_trait;
use omd::SharedOmdRuntime;
use serde_json::{json, Value};

use super::spec::{ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec};

// ══════════════════════════════════════════════════════════════
// omd_phase_complete
// ══════════════════════════════════════════════════════════════

pub struct OmdPhaseCompleteTool {
    runtime: SharedOmdRuntime,
}

impl OmdPhaseCompleteTool {
    pub fn new(runtime: SharedOmdRuntime) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl ToolSpec for OmdPhaseCompleteTool {
    fn name(&self) -> &str { "omd_phase_complete" }

    fn description(&self) -> &str {
        "Signal completion of current workflow phase and request transition to next phase. \
         Call when your current phase work is done. Tool availability changes on transition."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "next_phase": {"type": "string", "description": "Target phase (valid successor of current)"},
                "reason": {"type": "string", "description": "Why this phase is complete"},
                "evidence": {"type": "array", "items": {"type": "object"}, "description": "Evidence supporting transition"},
                "artifacts": {"type": "array", "items": {"type": "string"}, "description": "Files created/modified"}
            },
            "required": ["next_phase", "reason"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let next_phase = input.get("next_phase").and_then(|v| v.as_str()).unwrap_or("");
        let reason = input.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        let evidence = input.get("evidence").and_then(|v| v.as_array()).cloned().unwrap_or_default();

        // Verify evidence claims BEFORE accepting transition.
        // ExplicitSkip is NOT verified (it's an acknowledgment, not a verifiable claim).
        // It satisfies transition guards but is logged for audit.
        if !evidence.is_empty() {
            let state = self.runtime.read().await;
            let workspace = state.store.workspace();
            let audit_log = &state.audit_log;

            for ev_value in &evidence {
                // Try to parse as EvidenceClaim
                if let Ok(claim) = serde_json::from_value::<omd::EvidenceClaim>(ev_value.clone()) {
                    // ExplicitSkip bypasses verification — it's an explicit acknowledgment
                    // that evidence is unavailable. Must include a non-empty reason.
                    if let omd::EvidenceClaim::ExplicitSkip { ref reason } = claim {
                        if reason.trim().is_empty() {
                            return Err(ToolError::execution_failed(
                                "ExplicitSkip requires a non-empty reason explaining why \
                                 evidence is being skipped.".to_string()
                            ));
                        }
                        continue;
                    }
                    match omd::verify_claim(&claim, workspace, audit_log) {
                        Ok(omd::VerificationResult::Verified { .. }) => {},
                        Ok(omd::VerificationResult::RequiresUserAck { reason, .. }) => {
                            return Err(ToolError::execution_failed(format!(
                                "Evidence requires user acknowledgment before phase transition: {}. \
                                 Confirm via /omd-phase-complete.",
                                reason
                            )));
                        },
                        Err(reason) => {
                            return Err(ToolError::execution_failed(
                                format!("Evidence verification failed: {}", reason)
                            ));
                        }
                    }
                } else {
                    // Reject unparseable evidence — all evidence must be typed
                    return Err(ToolError::execution_failed(format!(
                        "Evidence claim could not be parsed as a valid EvidenceClaim type. \
                         Each evidence item must have a 'type' field (FileDiscovery, TestResult, \
                         GitDiff, PlanArtifact, or ExplicitSkip). Got: {}",
                        serde_json::to_string(ev_value).unwrap_or_else(|_| "???".to_string())
                    )));
                }
            }
            drop(state); // Release read lock before acquiring write lock
        }

        // Check transition-specific evidence requirements
        {
            let state = self.runtime.read().await;
            let current_phase = state.fsm.phase().clone();
            // Extract evidence type names from the evidence array
            let evidence_types: Vec<&str> = evidence.iter()
                .filter_map(|ev| ev.get("type").and_then(|t| t.as_str()))
                .collect();
            if let Err(missing) = omd::check_evidence_requirements(&current_phase, next_phase, &evidence_types) {
                let missing_names: Vec<&str> = missing.iter().map(|r| match r {
                    omd::RequiredEvidence::FileDiscovery => "FileDiscovery",
                    omd::RequiredEvidence::TestResult => "TestResult",
                    omd::RequiredEvidence::GitDiff => "GitDiff",
                    omd::RequiredEvidence::PlanArtifact => "PlanArtifact",
                }).collect();
                return Err(ToolError::execution_failed(format!(
                    "Transition from '{}' to '{}' requires evidence of type(s): {:?}. \
                     Provide the required evidence or use ExplicitSkip with a reason.",
                    current_phase.name(), next_phase, missing_names
                )));
            }
        }

        let mut state = self.runtime.write().await;
        let result = state.handle_phase_complete(next_phase, reason, &evidence);

        Ok(ToolResult {
            success: result.get("ok") == Some(&json!(true)),
            content: serde_json::to_string_pretty(&result).unwrap_or_default(),
            metadata: Some(result),
        })
    }
}

// ══════════════════════════════════════════════════════════════
// omd_checkpoint
// ══════════════════════════════════════════════════════════════

pub struct OmdCheckpointTool {
    runtime: SharedOmdRuntime,
}

impl OmdCheckpointTool {
    pub fn new(runtime: SharedOmdRuntime) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl ToolSpec for OmdCheckpointTool {
    fn name(&self) -> &str { "omd_checkpoint" }

    fn description(&self) -> &str {
        "Save a progress checkpoint within the current phase for session resumption."
    }

    fn input_schema(&self) -> Value {
        json!({"type": "object", "properties": {"summary": {"type": "string", "description": "Brief description of progress so far"}}, "required": ["summary"]})
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let summary = input.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.runtime.read().await;
        let result = state.handle_checkpoint(summary);
        Ok(ToolResult { success: true, content: "Checkpoint saved.".into(), metadata: Some(result) })
    }
}

// ══════════════════════════════════════════════════════════════
// omd_state_read
// ══════════════════════════════════════════════════════════════

pub struct OmdStateReadTool {
    runtime: SharedOmdRuntime,
}

impl OmdStateReadTool {
    pub fn new(runtime: SharedOmdRuntime) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl ToolSpec for OmdStateReadTool {
    fn name(&self) -> &str { "omd_state_read" }

    fn description(&self) -> &str {
        "Read current OMD workflow state: agent, phase, valid transitions, session info."
    }

    fn input_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(&self, _input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let state = self.runtime.read().await;
        let result = state.handle_state_read();
        Ok(ToolResult { success: true, content: serde_json::to_string_pretty(&result).unwrap_or_default(), metadata: Some(result) })
    }
}
