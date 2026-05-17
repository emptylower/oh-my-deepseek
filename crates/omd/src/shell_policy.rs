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

/// Patterns that indicate writes or shell escape even in otherwise-read commands.
/// NOTE: Command substitution ($(...), backticks) and awk/python system() calls
/// can execute arbitrary code. We block these at the string level since we don't
/// have a full shell parser (Plan 4 scope).
const WRITE_INDICATORS: &[&str] = &[
    ">", ">>",
    "|",  // Block ALL pipes
    "| sh", "| bash", "| zsh", "|sh", "|bash",
    "$(", "`",  // Command substitution — can execute arbitrary code
    "sed -i", "sed --in-place",
    "find -delete", "find -exec",
    "git branch -D", "git branch -d", "git reset", "git checkout --",
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
    // Check for write indicators / shell escape patterns first
    for indicator in WRITE_INDICATORS {
        if command.contains(indicator) {
            if *indicator == "|" {
                return Err(
                    "Piped commands are not supported in read-only mode. \
                     Use individual commands instead (pipe validation requires a full shell \
                     parser, which is Plan 4 scope).".to_string()
                );
            }
            return Err(format!(
                "Command contains write/escape indicator '{}'. Only read-only commands allowed.",
                indicator
            ));
        }
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

    let subcommands = split_command_chain(command);
    for subcmd in &subcommands {
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

fn split_command_chain(command: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut current = command;

    for delim in &["&&", "||", ";"] {
        let new_parts: Vec<&str> = if parts.is_empty() {
            vec![current]
        } else {
            parts.clone()
        };
        parts = new_parts.iter().flat_map(|p| p.split(delim)).collect();
        current = "";
    }

    if parts.is_empty() {
        vec![command]
    } else {
        parts
    }
}

fn is_allowed_read_command(command: &str) -> bool {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return true;
    }

    let binary = parts[0];
    let rest = parts[1..].join(" ");

    // Special case: find is allowed ONLY if no -exec/-delete/-ok
    if binary == "find" {
        return !rest.contains("-exec") && !rest.contains("-delete") && !rest.contains("-ok");
    }

    READ_ONLY_ALLOWLIST.iter().any(|(allowed_bin, prefix)| {
        if *allowed_bin != binary {
            return false;
        }
        if prefix.is_empty() {
            return true;
        }
        rest.starts_with(prefix)
    })
}
