use serde::{Deserialize, Serialize};
use std::path::Path;

/// Model-submitted evidence claims (Contract 4 from spec).
/// Client verifies before accepting a phase transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EvidenceClaim {
    FileDiscovery { paths: Vec<String> },
    TestResult { command: String, exit_code: i32, stdout_tail: Option<String> },
    GitDiff { changed_files: Vec<String> },
    PlanArtifact { path: String },
    ExplicitSkip { reason: String },
}

/// Result of evidence verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    Verified { method: String },
    RequiresUserAck { method: String, reason: String },
}

/// Client-verified evidence (stored in state after validation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedEvidence {
    pub claim: EvidenceClaim,
    pub verified_at: String,
    pub result: VerificationResult,
}

/// Verify an evidence claim against the filesystem/state.
/// `audit_log` is the list of (command, exit_code) tuples recently executed in this session.
/// Returns Ok(VerificationResult) on success, Err(reason) on failure.
pub fn verify_claim(
    claim: &EvidenceClaim,
    workspace: &Path,
    audit_log: &[(String, i32)],
) -> Result<VerificationResult, String> {
    match claim {
        EvidenceClaim::FileDiscovery { paths } => {
            if paths.is_empty() {
                return Err("FileDiscovery evidence must contain at least one path".to_string());
            }
            for path in paths {
                let full = if Path::new(path).is_absolute() {
                    std::path::PathBuf::from(path)
                } else {
                    workspace.join(path)
                };
                if !full.exists() {
                    return Err(format!("File not found: {}", path));
                }
            }
            Ok(VerificationResult::Verified { method: "fs_exists".to_string() })
        }
        EvidenceClaim::TestResult { command, exit_code, .. } => {
            if command.is_empty() {
                return Err("TestResult must specify command".to_string());
            }
            if *exit_code != 0 {
                return Err(format!("TestResult exit_code {} indicates failure", exit_code));
            }
            let found = audit_log.iter().any(|(cmd, code)| {
                cmd.as_str() == command.as_str() && *code == *exit_code
            });
            if !found {
                return Err(format!(
                    "TestResult command '{}' (exit_code={}) not found in shell audit log. \
                     The command must have been executed via exec_shell in this session with \
                     the exact same command string.",
                    command, exit_code
                ));
            }
            Ok(VerificationResult::Verified { method: "exec_audit_match".to_string() })
        }
        EvidenceClaim::GitDiff { changed_files } => {
            if changed_files.is_empty() {
                return Err("GitDiff must list at least one changed file".to_string());
            }
            for path in changed_files {
                let full = if Path::new(path).is_absolute() {
                    std::path::PathBuf::from(path)
                } else {
                    workspace.join(path)
                };
                if !full.exists() {
                    return Err(format!("Changed file not found: {}", path));
                }
            }
            Ok(VerificationResult::Verified { method: "git_diff_stat".to_string() })
        }
        EvidenceClaim::PlanArtifact { path } => {
            let full = if Path::new(path).is_absolute() {
                std::path::PathBuf::from(path)
            } else {
                workspace.join(path)
            };
            if !full.exists() {
                return Err(format!("Plan artifact not found: {}", path));
            }
            let content = std::fs::read_to_string(&full)
                .map_err(|e| format!("Cannot read plan: {}", e))?;
            if !content.contains("- [ ]") && !content.contains("- [x]") {
                return Err("Plan artifact must contain checkbox task markers (- [ ])".to_string());
            }
            Ok(VerificationResult::Verified { method: "plan_validated".to_string() })
        }
        EvidenceClaim::ExplicitSkip { reason } => {
            if reason.is_empty() {
                return Err("ExplicitSkip must provide a reason".to_string());
            }
            Ok(VerificationResult::RequiresUserAck {
                method: "explicit_skip".to_string(),
                reason: reason.clone(),
            })
        }
    }
}
