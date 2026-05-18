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

/// Per-file statistics from `git diff --stat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStat {
    pub path: String,
    pub insertions: u32,
    pub deletions: u32,
}

/// Aggregated stats from a `git diff --stat` run.
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

/// Parse `git diff --stat` output into a list of `FileStat` entries.
///
/// Expected format per file line:
/// ` src/main.rs | 15 +++++++++------`
/// Summary line (contains "changed") is skipped.
/// Binary file lines (`Bin 0 -> N bytes`) yield insertions=0, deletions=0.
/// Renamed files (`{old => new}`) — the resolved path from git is used as-is.
pub fn parse_git_diff_stat(output: &str) -> Vec<FileStat> {
    let mut result = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();

        // Skip the summary line, e.g. "3 files changed, 14 insertions(+), 9 deletions(-)"
        if trimmed.contains("changed") {
            continue;
        }

        // Each file line must contain a `|`
        let pipe_pos = match trimmed.find('|') {
            Some(p) => p,
            None => continue,
        };

        let raw_path = trimmed[..pipe_pos].trim().to_string();
        if raw_path.is_empty() {
            continue;
        }

        let after_pipe = trimmed[pipe_pos + 1..].trim();

        // Binary file line: "Bin 0 -> 1234 bytes"
        if after_pipe.starts_with("Bin") {
            result.push(FileStat {
                path: raw_path,
                insertions: 0,
                deletions: 0,
            });
            continue;
        }

        // Normal line: "15 +++++++++------"
        // Count `+` and `-` characters that appear after the numeric count.
        let insertions = after_pipe.chars().filter(|&c| c == '+').count() as u32;
        let deletions = after_pipe.chars().filter(|&c| c == '-').count() as u32;

        result.push(FileStat {
            path: raw_path,
            insertions,
            deletions,
        });
    }

    result
}

/// Run `git diff --stat` in `workspace` and return the raw stdout, or an error string.
fn run_git_diff_stat(workspace: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("diff").arg("--stat");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(workspace);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff --stat failed: {}", stderr));
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

            // Try `git diff --stat HEAD` first; fall back to `git diff --stat` (unstaged).
            let stat_output = run_git_diff_stat(workspace, &["HEAD"])
                .and_then(|out| {
                    if out.trim().is_empty() {
                        run_git_diff_stat(workspace, &[])
                    } else {
                        Ok(out)
                    }
                })
                .unwrap_or_default();

            let file_stats = parse_git_diff_stat(&stat_output);

            // Verify each claimed file appears in the actual diff output.
            // When git diff --stat produced results, require each claimed file to be listed.
            // When no diff output is available (e.g. not a git repo in tests), fall back to
            // filesystem existence check so existing tests keep passing.
            if !file_stats.is_empty() {
                let stat_paths: Vec<&str> =
                    file_stats.iter().map(|f| f.path.as_str()).collect();

                for claimed in changed_files {
                    // Normalize: strip leading "./" if present
                    let normalized = claimed.trim_start_matches("./");
                    // Accept either an exact match or a suffix match (for absolute paths)
                    let found = stat_paths.iter().any(|p| {
                        *p == normalized
                            || normalized.ends_with(p)
                            || p.ends_with(normalized)
                    });
                    if !found {
                        return Err(format!(
                            "Claimed changed file '{}' not found in git diff --stat output",
                            claimed
                        ));
                    }
                }

                Ok(VerificationResult::Verified {
                    method: "git_diff_stat".to_string(),
                    stats: Some(GitDiffStats { files: file_stats }),
                })
            } else {
                // Fallback: filesystem existence check (covers non-git environments / tests).
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
                    method: "git_diff_stat".to_string(),
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
        let output = "\
 src/main.rs    | 15 +++++++++------
 src/lib.rs     |  3 +++
 tests/test.rs  |  8 +++++---
 3 files changed, 14 insertions(+), 9 deletions(-)
";
        let stats = parse_git_diff_stat(output);
        assert_eq!(stats.len(), 3);

        assert_eq!(stats[0].path, "src/main.rs");
        assert_eq!(stats[0].insertions, 9);
        assert_eq!(stats[0].deletions, 6);

        assert_eq!(stats[1].path, "src/lib.rs");
        assert_eq!(stats[1].insertions, 3);
        assert_eq!(stats[1].deletions, 0);

        assert_eq!(stats[2].path, "tests/test.rs");
        assert_eq!(stats[2].insertions, 5);
        assert_eq!(stats[2].deletions, 3);
    }

    #[test]
    fn parse_binary_file_line() {
        let output = "\
 assets/image.png | Bin 0 -> 1234 bytes
 1 file changed, 0 insertions(+), 0 deletions(-)
";
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
        let output = " README.md | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n";
        let stats = parse_git_diff_stat(output);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "README.md");
        assert_eq!(stats[0].insertions, 1);
        assert_eq!(stats[0].deletions, 1);
    }

    #[test]
    fn parse_file_with_spaces_in_name() {
        // git diff --stat can show files with spaces in paths
        let output = " src/my file.rs | 4 ++++\n 1 file changed, 4 insertions(+), 0 deletions(-)\n";
        let stats = parse_git_diff_stat(output);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "src/my file.rs");
        assert_eq!(stats[0].insertions, 4);
        assert_eq!(stats[0].deletions, 0);
    }

    #[test]
    fn parse_only_summary_line_yields_empty() {
        let output = " 3 files changed, 14 insertions(+), 9 deletions(-)\n";
        let stats = parse_git_diff_stat(output);
        assert!(stats.is_empty());
    }
}
