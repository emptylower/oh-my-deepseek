use glob_match::glob_match;
use std::path::{Path, PathBuf};

/// Validates that a file path is within the allowed write scope.
/// Used by: omd_delegate (worker writes) and Fuxi Plan phase (plan writes).
pub struct WriteScopeValidator {
    patterns: Vec<String>,
    workspace: PathBuf,
}

impl WriteScopeValidator {
    pub fn new(patterns: &[&str]) -> Self {
        Self {
            patterns: patterns.iter().map(|s| s.to_string()).collect(),
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    pub fn with_workspace(patterns: &[&str], workspace: &Path) -> Self {
        Self {
            patterns: patterns.iter().map(|s| s.to_string()).collect(),
            workspace: workspace.to_path_buf(),
        }
    }

    pub fn from_strings(patterns: &[String]) -> Self {
        Self {
            patterns: patterns.to_vec(),
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    fn canonicalize_path(&self, path: &str) -> Option<String> {
        if path.contains("..") {
            return None;
        }

        let full = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.workspace.join(path)
        };

        let canonical_workspace = self.workspace.canonicalize().ok()?;

        let canonical = if let Ok(c) = full.canonicalize() {
            c
        } else {
            let mut existing_parent = full.parent();
            let mut remaining_segments = Vec::new();

            loop {
                match existing_parent {
                    Some(parent) => {
                        if let Ok(canonical_parent) = parent.canonicalize() {
                            let mut result = canonical_parent;
                            for seg in remaining_segments.iter().rev() {
                                result = result.join(seg);
                            }
                            break result;
                        }
                        if let Some(file_name) = parent.file_name() {
                            remaining_segments.push(file_name.to_os_string());
                        }
                        existing_parent = parent.parent();
                    }
                    None => return None,
                }
            }
        };

        if !canonical.starts_with(&canonical_workspace) {
            return None;
        }

        if Path::new(path).is_absolute() {
            full.strip_prefix(&self.workspace)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            Some(path.to_string())
        }
    }

    /// Return the configured glob patterns for this scope validator.
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// Check if a path is allowed by any pattern in the scope.
    /// Rejects path traversal attempts (../) and symlink escapes.
    pub fn is_allowed(&self, path: &str) -> bool {
        if self.patterns.is_empty() {
            return false;
        }

        let relative_path = match self.canonicalize_path(path) {
            Some(p) => p,
            None => return false,
        };

        self.patterns.iter().any(|pattern| glob_match(pattern, &relative_path))
    }
}
