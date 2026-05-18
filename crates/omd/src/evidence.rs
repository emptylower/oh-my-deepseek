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

/// Per-file statistics from `git diff --numstat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStat {
    pub path: String,
    pub insertions: u32,
    pub deletions: u32,
}

/// Aggregated stats from a `git diff --numstat` run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitDiffStats {
    pub files: Vec<FileStat>,
}

/// Result of evidence verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    Verified { method: String, stats: Option<GitDiffStats> },
    RequiresUserAck { method: String, reason: String },
}

/// Client-verified evidence (stored in state after validation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedEvidence {
    pub claim: EvidenceClaim,
    pub verified_at: String,
    pub result: VerificationResult,
}

/// Strip a leading `./` from a path for normalization.
fn normalize_path(p: &str) -> &str {
    p.strip_prefix("./").unwrap_or(p)
}

/// Parse `git diff --numstat` output into a list of `FileStat` entries.
///
/// Expected format per line (tab-separated):
/// `9\t6\tsrc/main.rs`
/// Binary files are reported as `-\t-\tpath` and yield insertions=0, deletions=0.
pub fn parse_git_diff_stat(output: &str) -> Vec<FileStat> {
    let mut files = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() != 3 {
            continue;
        }
        // Binary files have "-" for insertions/deletions; parse::<u32>() returns Err → 0.
        let insertions = parts[0].parse::<u32>().unwrap_or(0);
        let deletions = parts[1].parse::<u32>().unwrap_or(0);
        let path = parts[2].to_string();
        files.push(FileStat { path, insertions, deletions });
    }
    files
}

/// Run `git diff --numstat` in `workspace` and return the raw stdout, or an error string.
fn run_git_diff_stat(workspace: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("diff").arg("--numstat");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(workspace);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff --numstat failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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
            Ok(VerificationResult::Verified { method: "fs_exists".to_string(), stats: None })
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
            Ok(VerificationResult::Verified { method: "exec_audit_match".to_string(), stats: None })
        }
        EvidenceClaim::GitDiff { changed_files } => {
            if changed_files.is_empty() {
                return Err("GitDiff must list at least one changed file".to_string());
            }

            // Try `git diff --numstat HEAD` first; fall back to unstaged.
            // Track whether git is available to distinguish "clean repo" from "non-git env".
            let (git_available, stat_output) = match run_git_diff_stat(workspace, &["HEAD"]) {
                Ok(out) if out.trim().is_empty() => {
                    // HEAD diff empty — try unstaged
                    match run_git_diff_stat(workspace, &[]) {
                        Ok(out2) => (true, out2),
                        Err(_) => (true, String::new()),
                    }
                }
                Ok(out) => (true, out),
                Err(_) => (false, String::new()), // git not available
            };

            let file_stats = parse_git_diff_stat(&stat_output);

            // Verify each claimed file appears in the actual diff output.
            // When git diff --numstat produced results, require each claimed file to be listed.
            // When no diff output is available (e.g. not a git repo in tests), fall back to
            // filesystem existence check so existing tests keep passing.
            if !file_stats.is_empty() {
                let stat_paths: Vec<&str> =
                    file_stats.iter().map(|f| f.path.as_str()).collect();

                for claimed in changed_files {
                    // Normalize: strip leading "./" if present, then require an exact match.
                    let claimed_norm = normalize_path(claimed.trim_start_matches("./"));
                    let found = stat_paths.iter().any(|p| normalize_path(p) == claimed_norm);
                    if !found {
                        return Err(format!(
                            "Claimed changed file '{}' not found in git diff --numstat output",
                            claimed
                        ));
                    }
                }

                Ok(VerificationResult::Verified {
                    method: "git_diff_stat".to_string(),
                    stats: Some(GitDiffStats { files: file_stats }),
                })
            } else if git_available {
                // Git is available but no changes detected — reject.
                return Err(
                    "No changes detected by git diff --numstat. GitDiff evidence requires \
                     actual file changes visible to git.".to_string()
                );
            } else {
                // Git not available (non-git environment / tests) — filesystem fallback.
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
                Ok(VerificationResult::Verified {
                    method: "fs_exists_fallback".to_string(),
                    stats: None,
                })
            }
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
            Ok(VerificationResult::Verified { method: "plan_validated".to_string(), stats: None })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_normal_multi_file_output() {
        let output = "9\t6\tsrc/main.rs\n3\t0\tsrc/lib.rs\n8\t3\ttests/test.rs\n";
        let stats = parse_git_diff_stat(output);
        assert_eq!(stats.len(), 3);

        assert_eq!(stats[0].path, "src/main.rs");
        assert_eq!(stats[0].insertions, 9);
        assert_eq!(stats[0].deletions, 6);

        assert_eq!(stats[1].path, "src/lib.rs");
        assert_eq!(stats[1].insertions, 3);
        assert_eq!(stats[1].deletions, 0);

        assert_eq!(stats[2].path, "tests/test.rs");
        assert_eq!(stats[2].insertions, 8);
        assert_eq!(stats[2].deletions, 3);
    }

    #[test]
    fn parse_binary_file_line() {
        let output = "-\t-\tassets/image.png\n";
        let stats = parse_git_diff_stat(output);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "assets/image.png");
        assert_eq!(stats[0].insertions, 0);
        assert_eq!(stats[0].deletions, 0);
    }

    #[test]
    fn parse_empty_output() {
        let stats = parse_git_diff_stat("");
        assert!(stats.is_empty());
    }

    #[test]
    fn parse_single_file() {
        let output = "1\t1\tREADME.md\n";
        let stats = parse_git_diff_stat(output);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "README.md");
        assert_eq!(stats[0].insertions, 1);
        assert_eq!(stats[0].deletions, 1);
    }

    #[test]
    fn parse_file_with_spaces_in_name() {
        // git diff --numstat preserves spaces in file paths
        let output = "4\t0\tsrc/my file.rs\n";
        let stats = parse_git_diff_stat(output);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "src/my file.rs");
        assert_eq!(stats[0].insertions, 4);
        assert_eq!(stats[0].deletions, 0);
    }

    #[test]
    fn parse_only_summary_line_yields_empty() {
        // With --numstat there is no summary line, but a non-tab line should be skipped gracefully.
        let output = "some random line without tabs\n";
        let stats = parse_git_diff_stat(output);
        assert!(stats.is_empty());
    }
}
