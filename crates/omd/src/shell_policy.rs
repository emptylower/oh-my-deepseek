use crate::shell_parser;

/// Shell execution policy tiers (Contract 3 from spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellPolicy {
    /// No shell access at all
    None,
    /// Read-only commands only (pattern-matched allowlist)
    ReadOnly,
    /// Unrestricted shell
    Full,
}

/// Binary + optional subcommand prefix allowlist for ReadOnly mode.
const READ_ONLY_ALLOWLIST: &[(&str, &str)] = &[
    // Filesystem read
    ("cat", ""), ("head", ""), ("tail", ""), ("less", ""), ("wc", ""),
    ("sort", ""), ("uniq", ""), ("diff", ""), ("ls", ""), ("tree", ""),
    ("file", ""), ("stat", ""), ("du", ""),
    // Search
    ("grep", ""), ("rg", ""), ("ag", ""), ("fd", ""),
    // Text processing (read-only)
    ("awk", ""), ("jq", ""), ("yq", ""), ("column", ""), ("cut", ""), ("tr", ""),
    // Git (read-only subcommands)
    ("git", "log"), ("git", "diff"), ("git", "status"),
    ("git", "show"), ("git", "blame"), ("git", "rev-parse"),
    ("git", "branch --list"), ("git", "branch -a"), ("git", "branch -r"),
    ("git", "tag --list"), ("git", "tag -l"),
    // Build check (no output mutation)
    ("cargo", "check"), ("cargo", "clippy"),
    ("cargo", "test"),
    ("npx", "tsc --noEmit"),
    // Python
    ("python", "-m py_compile"), ("python3", "-m py_compile"),
    // General read
    ("echo", ""), ("printf", ""), ("env", ""), ("which", ""), ("whereis", ""),
    ("uname", ""), ("date", ""), ("id", ""), ("whoami", ""),
];

/// Validate a shell command against the given policy.
pub fn validate_command(command: &str, policy: ShellPolicy) -> Result<(), String> {
    match policy {
        ShellPolicy::Full => Ok(()),
        ShellPolicy::None => Err("Shell execution not available in this phase".to_string()),
        ShellPolicy::ReadOnly => validate_read_only(command),
    }
}

fn validate_read_only(command: &str) -> Result<(), String> {
    // Reject pipes (unquoted `|` not part of `||`)
    if shell_parser::has_pipe(command) {
        return Err(
            "Piped commands are not supported in read-only mode. \
             Use individual commands instead.".to_string()
        );
    }

    // Reject output redirects (`>`, `>>`)
    if shell_parser::has_redirect(command) {
        return Err(
            "Command contains write/escape indicator '>' or '>>'. Only read-only commands allowed."
                .to_string()
        );
    }

    // Reject command substitution (`$(...)`, backticks)
    if shell_parser::has_command_substitution(command) {
        return Err(
            "Command contains write/escape indicator '$(' or backtick. Only read-only commands allowed."
                .to_string()
        );
    }

    // Reject process substitution (`<(...)`, `>(...)`)
    if shell_parser::has_process_substitution(command) {
        return Err(
            "Command contains write/escape indicator '<(' or '>('. Only read-only commands allowed."
                .to_string()
        );
    }

    // Block awk/python system() calls that can execute arbitrary code
    if command.contains("system(") || command.contains("system (") {
        return Err(
            "Command contains system() call which can execute arbitrary code. \
             Not allowed in read-only mode.".to_string()
        );
    }

    // Block git diff --output (writes to file)
    if command.contains("--output=") || command.contains("--output ") {
        return Err(
            "Command contains --output which writes to a file. Not allowed in read-only mode."
                .to_string()
        );
    }

    // Split at unquoted &&, ||, ;, &, newline — then check each sub-command
    let subcommands = shell_parser::split_commands(command);
    for (_, subcmd) in &subcommands {
        let subcmd = subcmd.trim();
        if subcmd.is_empty() {
            continue;
        }
        if !is_allowed_read_command(subcmd) {
            return Err(format!(
                "Command '{}' not in read-only allowlist. Allowed binaries: cat, ls, find \
                 (no -exec/-delete/-ok), grep, awk, git log/diff/status, cargo check/test, etc.",
                subcmd
            ));
        }
    }

    Ok(())
}

fn is_allowed_read_command(command: &str) -> bool {
    // Use the parser to get proper argv tokens (handles quoting correctly)
    let parts = match shell_parser::tokenize(command) {
        Ok(p) => p,
        Err(_) => return false, // unterminated quote → reject
    };

    if parts.is_empty() {
        return true;
    }

    let binary = &parts[0];
    let rest = parts[1..].join(" ");

    // Special case: find is allowed ONLY if no -exec/-delete/-ok
    if binary == "find" {
        return !rest.contains("-exec") && !rest.contains("-delete") && !rest.contains("-ok");
    }

    // Special case: sed is NOT in the allowlist at all; always reject
    if binary == "sed" {
        return false;
    }

    READ_ONLY_ALLOWLIST.iter().any(|(allowed_bin, prefix)| {
        if allowed_bin != binary {
            return false;
        }
        if prefix.is_empty() {
            return true;
        }
        // Exact token match: prefix must match as a complete token boundary.
        if rest.starts_with(prefix) {
            let after = &rest[prefix.len()..];
            after.is_empty() || after.starts_with(' ')
        } else {
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_sub_inside_double_quotes_rejected() {
        // CRITICAL: $() inside double quotes must be caught
        assert!(validate_command(r#"echo "$(rm -rf /)""#, ShellPolicy::ReadOnly).is_err());
    }

    #[test]
    fn backtick_inside_double_quotes_rejected() {
        // CRITICAL: backticks inside double quotes must be caught
        assert!(validate_command(r#"echo "`rm -rf /`""#, ShellPolicy::ReadOnly).is_err());
    }

    #[test]
    fn safe_echo_allowed() {
        assert!(validate_command("echo hello", ShellPolicy::ReadOnly).is_ok());
    }

    #[test]
    fn pipe_rejected_with_helpful_message() {
        let err = validate_command("ls | wc -l", ShellPolicy::ReadOnly).unwrap_err();
        assert!(err.contains("Piped commands are not supported"));
        assert!(!err.contains("Plan 4 scope"));
    }
}
