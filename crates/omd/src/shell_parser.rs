/// Shell argv tokenizer — Contract 3 full implementation.
///
/// Handles:
/// - Single quotes (no escaping inside)
/// - Double quotes (backslash escapes inside)
/// - Backslash escaping outside quotes
/// - Chain operators: `&&`, `||`, `;`
/// - Background operator: `&`
/// - Newline separation
/// - Pipes: `|`
/// - Output redirects: `>`, `>>`
/// - Command substitution: `$(...)`, backticks
/// - Process substitution: `<(...)`, `>(...)`
/// - Unterminated quotes → error

/// Operators that separate sub-commands in a shell command chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    /// First command (no preceding operator)
    First,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `;`
    Semi,
    /// `&` (background)
    Background,
    /// newline
    Newline,
}

/// Tokenize a shell command string into argv tokens.
///
/// Returns `Err` if the input contains an unterminated quote.
/// Operators (`&&`, `||`, `;`, `&`, `|`, `>`, `>>`, newlines) are NOT
/// included as tokens — they act only as delimiters or are detected via
/// the `has_*` family of functions.
pub fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        match ch {
            // Single-quote: everything until next `'` is literal
            '\'' => {
                in_token = true;
                i += 1;
                loop {
                    if i >= len {
                        return Err("Unterminated single quote".to_string());
                    }
                    if chars[i] == '\'' {
                        i += 1;
                        break;
                    }
                    current.push(chars[i]);
                    i += 1;
                }
            }

            // Double-quote: backslash escapes apply
            '"' => {
                in_token = true;
                i += 1;
                loop {
                    if i >= len {
                        return Err("Unterminated double quote".to_string());
                    }
                    match chars[i] {
                        '"' => {
                            i += 1;
                            break;
                        }
                        '\\' if i + 1 < len => {
                            // Only \, ", $, `, and newline are special inside double quotes
                            let next = chars[i + 1];
                            match next {
                                '\\' | '"' | '$' | '`' | '\n' => {
                                    current.push(next);
                                    i += 2;
                                }
                                _ => {
                                    current.push('\\');
                                    i += 1;
                                }
                            }
                        }
                        c => {
                            current.push(c);
                            i += 1;
                        }
                    }
                }
            }

            // Backslash outside quotes: next char is literal
            '\\' => {
                in_token = true;
                if i + 1 < len {
                    current.push(chars[i + 1]);
                    i += 2;
                } else {
                    // Trailing backslash: treat as literal
                    current.push('\\');
                    i += 1;
                }
            }

            // Whitespace: ends the current token
            ' ' | '\t' => {
                if in_token {
                    tokens.push(current.clone());
                    current.clear();
                    in_token = false;
                }
                i += 1;
            }

            // Newline: ends token and acts as a command separator
            '\n' => {
                if in_token {
                    tokens.push(current.clone());
                    current.clear();
                    in_token = false;
                }
                i += 1;
            }

            // `&&` or `&`
            '&' => {
                if in_token {
                    tokens.push(current.clone());
                    current.clear();
                    in_token = false;
                }
                if i + 1 < len && chars[i + 1] == '&' {
                    i += 2;
                } else {
                    i += 1;
                }
            }

            // `||` or `|`
            '|' => {
                if in_token {
                    tokens.push(current.clone());
                    current.clear();
                    in_token = false;
                }
                if i + 1 < len && chars[i + 1] == '|' {
                    i += 2;
                } else {
                    i += 1;
                }
            }

            // `;`
            ';' => {
                if in_token {
                    tokens.push(current.clone());
                    current.clear();
                    in_token = false;
                }
                i += 1;
            }

            // `>(...)` process substitution, `>>`, or `>`
            '>' => {
                if in_token {
                    tokens.push(current.clone());
                    current.clear();
                    in_token = false;
                }
                if i + 1 < len && chars[i + 1] == '(' {
                    // process substitution >(...) — skip the block
                    i += 2;
                    let mut depth = 1usize;
                    while i < len && depth > 0 {
                        match chars[i] {
                            '(' => { depth += 1; i += 1; }
                            ')' => { depth -= 1; i += 1; }
                            '\'' => {
                                i += 1;
                                while i < len && chars[i] != '\'' { i += 1; }
                                if i < len { i += 1; }
                            }
                            '"' => {
                                i += 1;
                                while i < len && chars[i] != '"' {
                                    if chars[i] == '\\' { i += 1; }
                                    i += 1;
                                }
                                if i < len { i += 1; }
                            }
                            _ => { i += 1; }
                        }
                    }
                } else if i + 1 < len && chars[i + 1] == '>' {
                    i += 2; // >>
                } else {
                    i += 1; // >
                }
            }

            // `<(` process substitution, otherwise not a separator (e.g., stdin redirect)
            '<' => {
                // Process substitution <(...) — skip as meta; don't add to token stream
                if i + 1 < len && chars[i + 1] == '(' {
                    if in_token {
                        tokens.push(current.clone());
                        current.clear();
                        in_token = false;
                    }
                    // Skip over the <( ... ) block
                    i += 2; // skip `<(`
                    let mut depth = 1usize;
                    while i < len && depth > 0 {
                        match chars[i] {
                            '(' => { depth += 1; i += 1; }
                            ')' => { depth -= 1; i += 1; }
                            '\'' => {
                                i += 1;
                                while i < len && chars[i] != '\'' { i += 1; }
                                if i < len { i += 1; }
                            }
                            '"' => {
                                i += 1;
                                while i < len && chars[i] != '"' {
                                    if chars[i] == '\\' { i += 1; }
                                    i += 1;
                                }
                                if i < len { i += 1; }
                            }
                            _ => { i += 1; }
                        }
                    }
                } else {
                    // Plain `<` redirect — treat as separator (drop)
                    if in_token {
                        tokens.push(current.clone());
                        current.clear();
                        in_token = false;
                    }
                    i += 1;
                }
            }

            // `$(...)` command substitution or `>(...) ` process substitution
            '$' => {
                if i + 1 < len && chars[i + 1] == '(' {
                    // Command substitution $(...) — skip block
                    if in_token {
                        tokens.push(current.clone());
                        current.clear();
                        in_token = false;
                    }
                    i += 2;
                    let mut depth = 1usize;
                    while i < len && depth > 0 {
                        match chars[i] {
                            '(' => { depth += 1; i += 1; }
                            ')' => { depth -= 1; i += 1; }
                            '\'' => {
                                i += 1;
                                while i < len && chars[i] != '\'' { i += 1; }
                                if i < len { i += 1; }
                            }
                            '"' => {
                                i += 1;
                                while i < len && chars[i] != '"' {
                                    if chars[i] == '\\' { i += 1; }
                                    i += 1;
                                }
                                if i < len { i += 1; }
                            }
                            _ => { i += 1; }
                        }
                    }
                } else {
                    // $VAR or similar — treat as regular chars
                    current.push(ch);
                    in_token = true;
                    i += 1;
                }
            }

            // Backtick command substitution
            '`' => {
                if in_token {
                    tokens.push(current.clone());
                    current.clear();
                    in_token = false;
                }
                // Skip to closing backtick
                i += 1;
                while i < len && chars[i] != '`' {
                    if chars[i] == '\\' { i += 1; }
                    i += 1;
                }
                if i < len { i += 1; } // skip closing `
            }

            // Regular character
            c => {
                current.push(c);
                in_token = true;
                i += 1;
            }
        }
    }

    if in_token {
        tokens.push(current);
    }

    Ok(tokens)
}

/// Split a shell command string into sub-commands at unquoted `&&`, `||`, `;`, `&`, and newline.
///
/// Returns a list of `(Operator, raw_substring)` pairs.
/// The `Operator` describes what preceded this sub-command.
pub fn split_commands(input: &str) -> Vec<(Operator, String)> {
    let mut result: Vec<(Operator, String)> = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut segment_start = 0;
    let mut op = Operator::First;

    while i < len {
        let ch = chars[i];

        match ch {
            '\'' => {
                i += 1;
                while i < len && chars[i] != '\'' {
                    i += 1;
                }
                if i < len { i += 1; }
            }

            '"' => {
                i += 1;
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len { i += 1; }
                    i += 1;
                }
                if i < len { i += 1; }
            }

            '\\' => {
                i += 2; // skip escaped char
            }

            '$' if i + 1 < len && chars[i + 1] == '(' => {
                i += 2;
                let mut depth = 1usize;
                while i < len && depth > 0 {
                    match chars[i] {
                        '(' => { depth += 1; i += 1; }
                        ')' => { depth -= 1; i += 1; }
                        '\'' => { i += 1; while i < len && chars[i] != '\'' { i += 1; } if i < len { i += 1; } }
                        '"' => { i += 1; while i < len && chars[i] != '"' { if chars[i] == '\\' { i += 1; } i += 1; } if i < len { i += 1; } }
                        _ => { i += 1; }
                    }
                }
            }

            '`' => {
                i += 1;
                while i < len && chars[i] != '`' {
                    if chars[i] == '\\' { i += 1; }
                    i += 1;
                }
                if i < len { i += 1; }
            }

            '<' if i + 1 < len && chars[i + 1] == '(' => {
                i += 2;
                let mut depth = 1usize;
                while i < len && depth > 0 {
                    match chars[i] {
                        '(' => { depth += 1; i += 1; }
                        ')' => { depth -= 1; i += 1; }
                        '\'' => { i += 1; while i < len && chars[i] != '\'' { i += 1; } if i < len { i += 1; } }
                        '"' => { i += 1; while i < len && chars[i] != '"' { if chars[i] == '\\' { i += 1; } i += 1; } if i < len { i += 1; } }
                        _ => { i += 1; }
                    }
                }
            }

            '>' if i + 1 < len && chars[i + 1] == '(' => {
                i += 2;
                let mut depth = 1usize;
                while i < len && depth > 0 {
                    match chars[i] {
                        '(' => { depth += 1; i += 1; }
                        ')' => { depth -= 1; i += 1; }
                        '\'' => { i += 1; while i < len && chars[i] != '\'' { i += 1; } if i < len { i += 1; } }
                        '"' => { i += 1; while i < len && chars[i] != '"' { if chars[i] == '\\' { i += 1; } i += 1; } if i < len { i += 1; } }
                        _ => { i += 1; }
                    }
                }
            }

            '&' => {
                let segment: String = chars[segment_start..i].iter().collect();
                let trimmed = segment.trim().to_string();
                if !trimmed.is_empty() {
                    result.push((op.clone(), trimmed));
                }
                if i + 1 < len && chars[i + 1] == '&' {
                    op = Operator::And;
                    i += 2;
                } else {
                    op = Operator::Background;
                    i += 1;
                }
                segment_start = i;
            }

            '|' => {
                // `||` is a separator; bare `|` (pipe) is NOT a command separator
                if i + 1 < len && chars[i + 1] == '|' {
                    let segment: String = chars[segment_start..i].iter().collect();
                    let trimmed = segment.trim().to_string();
                    if !trimmed.is_empty() {
                        result.push((op.clone(), trimmed));
                    }
                    op = Operator::Or;
                    i += 2;
                    segment_start = i;
                } else {
                    // Plain pipe — not a command boundary
                    i += 1;
                }
            }

            ';' => {
                let segment: String = chars[segment_start..i].iter().collect();
                let trimmed = segment.trim().to_string();
                if !trimmed.is_empty() {
                    result.push((op.clone(), trimmed));
                }
                op = Operator::Semi;
                i += 1;
                segment_start = i;
            }

            '\n' => {
                let segment: String = chars[segment_start..i].iter().collect();
                let trimmed = segment.trim().to_string();
                if !trimmed.is_empty() {
                    result.push((op.clone(), trimmed));
                }
                op = Operator::Newline;
                i += 1;
                segment_start = i;
            }

            _ => {
                i += 1;
            }
        }
    }

    // Push trailing segment
    let segment: String = chars[segment_start..].iter().collect();
    let trimmed = segment.trim().to_string();
    if !trimmed.is_empty() {
        result.push((op, trimmed));
    }

    result
}

/// Returns `true` if `pattern` appears unquoted in `input`.
///
/// This is a character-level scan that respects single/double quotes and backslash escaping.
pub fn has_unquoted(input: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    let chars: Vec<char> = input.chars().collect();
    let pat: Vec<char> = pattern.chars().collect();
    let len = chars.len();
    let plen = pat.len();
    let mut i = 0;

    while i < len {
        match chars[i] {
            '\'' => {
                i += 1;
                while i < len && chars[i] != '\'' { i += 1; }
                if i < len { i += 1; }
            }
            '"' => {
                i += 1;
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len { i += 1; }
                    i += 1;
                }
                if i < len { i += 1; }
            }
            '\\' => {
                i += 2;
            }
            _ => {
                // Try to match pattern at position i
                if i + plen <= len {
                    let slice: String = chars[i..i + plen].iter().collect();
                    if slice == pattern {
                        return true;
                    }
                }
                i += 1;
            }
        }
    }
    false
}

/// Returns `true` if the input contains an unquoted pipe (`|` not part of `||`).
pub fn has_pipe(input: &str) -> bool {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        match chars[i] {
            '\'' => {
                i += 1;
                while i < len && chars[i] != '\'' { i += 1; }
                if i < len { i += 1; }
            }
            '"' => {
                i += 1;
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len { i += 1; }
                    i += 1;
                }
                if i < len { i += 1; }
            }
            '\\' => {
                i += 2;
            }
            '|' => {
                // `||` is NOT a pipe
                if i + 1 < len && chars[i + 1] == '|' {
                    i += 2;
                } else {
                    return true;
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    false
}

/// Returns `true` if the input contains an unquoted output redirect (`>` or `>>`).
pub fn has_redirect(input: &str) -> bool {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        match chars[i] {
            '\'' => {
                i += 1;
                while i < len && chars[i] != '\'' { i += 1; }
                if i < len { i += 1; }
            }
            '"' => {
                i += 1;
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len { i += 1; }
                    i += 1;
                }
                if i < len { i += 1; }
            }
            '\\' => {
                i += 2;
            }
            '>' => {
                // `>(...)` is process substitution, not a redirect
                if i + 1 < len && chars[i + 1] == '(' {
                    i += 2;
                    let mut depth = 1usize;
                    while i < len && depth > 0 {
                        match chars[i] {
                            '(' => { depth += 1; i += 1; }
                            ')' => { depth -= 1; i += 1; }
                            _ => { i += 1; }
                        }
                    }
                } else {
                    return true;
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    false
}

/// Returns `true` if the input contains unquoted command substitution (`$(...)` or backticks).
pub fn has_command_substitution(input: &str) -> bool {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        match chars[i] {
            '\'' => {
                i += 1;
                while i < len && chars[i] != '\'' { i += 1; }
                if i < len { i += 1; }
            }
            '"' => {
                i += 1;
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len { i += 1; }
                    i += 1;
                }
                if i < len { i += 1; }
            }
            '\\' => {
                i += 2;
            }
            '$' => {
                if i + 1 < len && chars[i + 1] == '(' {
                    return true;
                }
                i += 1;
            }
            '`' => {
                return true;
            }
            _ => {
                i += 1;
            }
        }
    }
    false
}

/// Returns `true` if the input contains unquoted process substitution (`<(...)` or `>(...)`).
pub fn has_process_substitution(input: &str) -> bool {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        match chars[i] {
            '\'' => {
                i += 1;
                while i < len && chars[i] != '\'' { i += 1; }
                if i < len { i += 1; }
            }
            '"' => {
                i += 1;
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len { i += 1; }
                    i += 1;
                }
                if i < len { i += 1; }
            }
            '\\' => {
                i += 2;
            }
            '<' => {
                if i + 1 < len && chars[i + 1] == '(' {
                    return true;
                }
                i += 1;
            }
            '>' => {
                if i + 1 < len && chars[i + 1] == '(' {
                    return true;
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── tokenize ───────────────────────────────────────────────────────────

    #[test]
    fn tokenize_empty() {
        assert_eq!(tokenize("").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn tokenize_simple() {
        assert_eq!(tokenize("cat src/main.rs").unwrap(), vec!["cat", "src/main.rs"]);
    }

    #[test]
    fn tokenize_single_quotes() {
        // Operator inside single quotes is literal
        assert_eq!(tokenize("echo 'hello && world'").unwrap(), vec!["echo", "hello && world"]);
    }

    #[test]
    fn tokenize_double_quotes() {
        assert_eq!(tokenize(r#"echo "hello | world""#).unwrap(), vec!["echo", "hello | world"]);
    }

    #[test]
    fn tokenize_double_quote_backslash_escape() {
        // \" inside double quotes → "
        assert_eq!(tokenize(r#"echo "say \"hi\"""#).unwrap(), vec!["echo", r#"say "hi""#]);
    }

    #[test]
    fn tokenize_backslash_outside() {
        // \space → literal space in token
        assert_eq!(tokenize(r"echo hello\ world").unwrap(), vec!["echo", "hello world"]);
    }

    #[test]
    fn tokenize_operator_and_splits() {
        // && splits into separate argv groups — tokenize returns tokens for ONE command
        // Let's check it does not include && as a token
        let tokens = tokenize("git log --oneline").unwrap();
        assert_eq!(tokens, vec!["git", "log", "--oneline"]);
    }

    #[test]
    fn tokenize_semicolons_stripped() {
        // tokenize drops ; as separator
        let tokens = tokenize("ls ; echo done").unwrap();
        assert_eq!(tokens, vec!["ls", "echo", "done"]);
    }

    #[test]
    fn tokenize_newline_separation() {
        let tokens = tokenize("ls\necho done").unwrap();
        assert_eq!(tokens, vec!["ls", "echo", "done"]);
    }

    #[test]
    fn tokenize_background_op() {
        // & stripped
        let tokens = tokenize("sleep 10 &").unwrap();
        assert_eq!(tokens, vec!["sleep", "10"]);
    }

    #[test]
    fn tokenize_process_substitution_lt() {
        // <(...) is skipped entirely
        let tokens = tokenize("diff <(cat a) <(cat b)").unwrap();
        assert_eq!(tokens, vec!["diff"]);
    }

    #[test]
    fn tokenize_command_substitution_dollar() {
        // $(...) is skipped
        let tokens = tokenize("echo $(hostname)").unwrap();
        assert_eq!(tokens, vec!["echo"]);
    }

    #[test]
    fn tokenize_backtick_stripped() {
        let tokens = tokenize("echo `whoami`").unwrap();
        assert_eq!(tokens, vec!["echo"]);
    }

    #[test]
    fn tokenize_unterminated_single_quote_error() {
        assert!(tokenize("echo 'unterminated").is_err());
    }

    #[test]
    fn tokenize_unterminated_double_quote_error() {
        assert!(tokenize(r#"echo "unterminated"#).is_err());
    }

    #[test]
    fn tokenize_escaped_operator_in_arg() {
        // \; should be literal semicolon in token
        let tokens = tokenize(r"find . -name \;").unwrap();
        assert_eq!(tokens, vec!["find", ".", "-name", ";"]);
    }

    #[test]
    fn tokenize_process_substitution_gt() {
        // >(…) is skipped entirely — only the command before it is kept
        let tokens = tokenize("tee >(wc -l)").unwrap();
        assert_eq!(tokens, vec!["tee"]);
    }

    // ── split_commands ─────────────────────────────────────────────────────

    #[test]
    fn split_single_command() {
        let cmds = split_commands("cat foo.txt");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].0, Operator::First);
        assert_eq!(cmds[0].1, "cat foo.txt");
    }

    #[test]
    fn split_and_operator() {
        let cmds = split_commands("ls && echo done");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].0, Operator::First);
        assert_eq!(cmds[1].0, Operator::And);
        assert_eq!(cmds[1].1, "echo done");
    }

    #[test]
    fn split_or_operator() {
        let cmds = split_commands("false || echo fallback");
        assert_eq!(cmds[1].0, Operator::Or);
        assert_eq!(cmds[1].1, "echo fallback");
    }

    #[test]
    fn split_semicolon() {
        let cmds = split_commands("ls; echo done");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[1].0, Operator::Semi);
    }

    #[test]
    fn split_background() {
        let cmds = split_commands("sleep 10 & echo hi");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[1].0, Operator::Background);
    }

    #[test]
    fn split_newline() {
        let cmds = split_commands("ls\necho done");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[1].0, Operator::Newline);
    }

    #[test]
    fn split_operator_inside_single_quotes_not_split() {
        // The `&&` inside single quotes should NOT split
        let cmds = split_commands("echo 'a && b'");
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn split_operator_inside_double_quotes_not_split() {
        let cmds = split_commands(r#"echo "a || b""#);
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn split_pipe_not_command_boundary() {
        // `|` (bare pipe) does NOT create a new sub-command entry
        let cmds = split_commands("ls | wc -l");
        // The whole "ls | wc -l" is one segment (pipe is not a command separator here)
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].1.contains("|"));
    }

    #[test]
    fn split_process_sub_lt_with_quoted_paren() {
        // The ')' inside single quotes must NOT prematurely close the <(…) block
        let cmds = split_commands("diff <(grep ')' file)");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].1, "diff <(grep ')' file)");
    }

    // ── has_pipe ───────────────────────────────────────────────────────────

    #[test]
    fn has_pipe_detects_plain_pipe() {
        assert!(has_pipe("ls | wc -l"));
    }

    #[test]
    fn has_pipe_ignores_or_operator() {
        assert!(!has_pipe("false || true"));
    }

    #[test]
    fn has_pipe_false_for_no_pipe() {
        assert!(!has_pipe("ls -la"));
    }

    #[test]
    fn has_pipe_inside_quotes_ignored() {
        assert!(!has_pipe("echo 'a | b'"));
        assert!(!has_pipe(r#"echo "a | b""#));
    }

    // ── has_redirect ───────────────────────────────────────────────────────

    #[test]
    fn has_redirect_detects_gt() {
        assert!(has_redirect("echo hello > file.txt"));
    }

    #[test]
    fn has_redirect_detects_append() {
        assert!(has_redirect("echo hello >> file.txt"));
    }

    #[test]
    fn has_redirect_ignores_process_sub() {
        // >(cmd) is process substitution, not a redirect
        assert!(!has_redirect("tee >(cat)"));
    }

    #[test]
    fn has_redirect_inside_quotes_ignored() {
        assert!(!has_redirect("echo '> not redirect'"));
    }

    // ── has_command_substitution ───────────────────────────────────────────

    #[test]
    fn has_command_sub_dollar_paren() {
        assert!(has_command_substitution("echo $(hostname)"));
    }

    #[test]
    fn has_command_sub_backtick() {
        assert!(has_command_substitution("echo `whoami`"));
    }

    #[test]
    fn has_command_sub_false_when_none() {
        assert!(!has_command_substitution("echo hello"));
    }

    #[test]
    fn has_command_sub_inside_quotes_ignored() {
        // Inside single quotes, $(...) is literal
        assert!(!has_command_substitution("echo '$(rm -rf /)'"));
    }

    // ── has_process_substitution ───────────────────────────────────────────

    #[test]
    fn has_process_sub_lt_paren() {
        assert!(has_process_substitution("diff <(cat a) <(cat b)"));
    }

    #[test]
    fn has_process_sub_gt_paren() {
        assert!(has_process_substitution("tee >(wc -l)"));
    }

    #[test]
    fn has_process_sub_false_when_none() {
        assert!(!has_process_substitution("echo hello"));
    }

    #[test]
    fn has_process_sub_inside_quotes_ignored() {
        assert!(!has_process_substitution("echo '<(not sub)'"  ));
    }

    // ── has_unquoted ───────────────────────────────────────────────────────

    #[test]
    fn has_unquoted_finds_pattern() {
        assert!(has_unquoted("echo hello", "hello"));
    }

    #[test]
    fn has_unquoted_not_inside_single_quotes() {
        assert!(!has_unquoted("echo 'hello'", "hello"));
    }

    #[test]
    fn has_unquoted_not_inside_double_quotes() {
        assert!(!has_unquoted(r#"echo "hello""#, "hello"));
    }
}
