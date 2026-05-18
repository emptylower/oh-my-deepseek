use omd::shell_parser::{tokenize, split_commands, has_unquoted, has_pipe, has_redirect,
                         has_command_substitution, has_process_substitution, Operator};

// ── tokenize ──────────────────────────────────────────────────────────────────

#[test]
fn tok_empty_input() {
    assert_eq!(tokenize("").unwrap(), Vec::<String>::new());
}

#[test]
fn tok_simple_argv() {
    assert_eq!(tokenize("cat src/main.rs").unwrap(), vec!["cat", "src/main.rs"]);
}

#[test]
fn tok_multi_space() {
    assert_eq!(tokenize("ls   -la   /tmp").unwrap(), vec!["ls", "-la", "/tmp"]);
}

#[test]
fn tok_single_quote_preserves_operators() {
    // Operators inside single quotes are literal
    assert_eq!(
        tokenize("echo 'hello && world'").unwrap(),
        vec!["echo", "hello && world"]
    );
}

#[test]
fn tok_double_quote_preserves_operators() {
    assert_eq!(
        tokenize(r#"echo "foo | bar""#).unwrap(),
        vec!["echo", "foo | bar"]
    );
}

#[test]
fn tok_double_quote_backslash_escape() {
    assert_eq!(
        tokenize(r#"echo "say \"hi\"""#).unwrap(),
        vec!["echo", r#"say "hi""#]
    );
}

#[test]
fn tok_backslash_outside_quotes_escapes_space() {
    assert_eq!(
        tokenize(r"my\ command arg").unwrap(),
        vec!["my command", "arg"]
    );
}

#[test]
fn tok_escaped_operator_is_literal() {
    // \; should be a literal semicolon, not a separator
    let tokens = tokenize(r"find . -name \;").unwrap();
    assert_eq!(tokens, vec!["find", ".", "-name", ";"]);
}

#[test]
fn tok_newline_acts_as_separator() {
    let tokens = tokenize("ls\necho done").unwrap();
    assert_eq!(tokens, vec!["ls", "echo", "done"]);
}

#[test]
fn tok_background_ampersand_stripped() {
    assert_eq!(tokenize("sleep 10 &").unwrap(), vec!["sleep", "10"]);
}

#[test]
fn tok_and_and_operator_stripped() {
    // && is a separator — tokenize yields tokens from both sides (operators removed)
    let tokens = tokenize("ls && echo done").unwrap();
    assert_eq!(tokens, vec!["ls", "echo", "done"]);
}

#[test]
fn tok_semicolon_separator_stripped() {
    let tokens = tokenize("ls; echo done").unwrap();
    assert_eq!(tokens, vec!["ls", "echo", "done"]);
}

#[test]
fn tok_process_sub_lt_skipped() {
    let tokens = tokenize("diff <(cat a) <(cat b)").unwrap();
    assert_eq!(tokens, vec!["diff"]);
}

#[test]
fn tok_command_sub_dollar_skipped() {
    let tokens = tokenize("echo $(hostname)").unwrap();
    assert_eq!(tokens, vec!["echo"]);
}

#[test]
fn tok_backtick_sub_skipped() {
    let tokens = tokenize("echo `whoami`").unwrap();
    assert_eq!(tokens, vec!["echo"]);
}

#[test]
fn tok_unterminated_single_quote_error() {
    assert!(tokenize("echo 'unterminated").is_err());
}

#[test]
fn tok_unterminated_double_quote_error() {
    assert!(tokenize(r#"echo "unterminated"#).is_err());
}

// ── split_commands ────────────────────────────────────────────────────────────

#[test]
fn split_single_cmd() {
    let cmds = split_commands("cat foo.txt");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].0, Operator::First);
    assert_eq!(cmds[0].1, "cat foo.txt");
}

#[test]
fn split_and_and() {
    let cmds = split_commands("ls && echo done");
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].0, Operator::First);
    assert_eq!(cmds[1].0, Operator::And);
    assert_eq!(cmds[1].1, "echo done");
}

#[test]
fn split_or_or() {
    let cmds = split_commands("false || echo fallback");
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[1].0, Operator::Or);
    assert_eq!(cmds[1].1, "echo fallback");
}

#[test]
fn split_semicolon() {
    let cmds = split_commands("ls; echo done");
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[1].0, Operator::Semi);
    assert_eq!(cmds[1].1, "echo done");
}

#[test]
fn split_background_ampersand() {
    let cmds = split_commands("sleep 10 & echo hi");
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[1].0, Operator::Background);
    assert_eq!(cmds[1].1, "echo hi");
}

#[test]
fn split_newline_operator() {
    let cmds = split_commands("ls\necho done");
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[1].0, Operator::Newline);
}

#[test]
fn split_operator_inside_single_quotes_not_split() {
    let cmds = split_commands("echo 'a && b'");
    assert_eq!(cmds.len(), 1, "operator inside single quotes must not split");
}

#[test]
fn split_operator_inside_double_quotes_not_split() {
    let cmds = split_commands(r#"echo "a || b""#);
    assert_eq!(cmds.len(), 1, "operator inside double quotes must not split");
}

#[test]
fn split_bare_pipe_not_a_boundary() {
    // `|` alone is NOT a command separator in split_commands
    let cmds = split_commands("ls | wc -l");
    assert_eq!(cmds.len(), 1);
}

#[test]
fn split_empty_segments_skipped() {
    // Trailing semicolon should not create an empty second entry
    let cmds = split_commands("ls;");
    assert_eq!(cmds.len(), 1);
}

// ── has_pipe ──────────────────────────────────────────────────────────────────

#[test]
fn pipe_detects_bare_pipe() {
    assert!(has_pipe("ls | wc -l"));
}

#[test]
fn pipe_or_operator_is_not_pipe() {
    assert!(!has_pipe("false || true"));
}

#[test]
fn pipe_inside_single_quotes_ignored() {
    assert!(!has_pipe("echo 'a | b'"));
}

#[test]
fn pipe_inside_double_quotes_ignored() {
    assert!(!has_pipe(r#"echo "a | b""#));
}

#[test]
fn pipe_false_when_absent() {
    assert!(!has_pipe("ls -la"));
}

// ── has_redirect ──────────────────────────────────────────────────────────────

#[test]
fn redirect_detects_gt() {
    assert!(has_redirect("echo hello > file.txt"));
}

#[test]
fn redirect_detects_gt_gt() {
    assert!(has_redirect("echo hello >> file.txt"));
}

#[test]
fn redirect_ignores_process_sub_gt() {
    // >(cmd) is process substitution, NOT a redirect
    assert!(!has_redirect("tee >(cat)"));
}

#[test]
fn redirect_inside_single_quotes_ignored() {
    assert!(!has_redirect("echo '> not a redirect'"));
}

#[test]
fn redirect_false_when_absent() {
    assert!(!has_redirect("cat file.txt"));
}

// ── has_command_substitution ──────────────────────────────────────────────────

#[test]
fn cmdsub_dollar_paren() {
    assert!(has_command_substitution("echo $(hostname)"));
}

#[test]
fn cmdsub_backtick() {
    assert!(has_command_substitution("echo `whoami`"));
}

#[test]
fn cmdsub_false_when_none() {
    assert!(!has_command_substitution("echo hello"));
}

#[test]
fn cmdsub_inside_single_quotes_ignored() {
    assert!(!has_command_substitution("echo '$(rm -rf /)'"));
}

// ── has_process_substitution ──────────────────────────────────────────────────

#[test]
fn procsub_lt_paren() {
    assert!(has_process_substitution("diff <(cat a) <(cat b)"));
}

#[test]
fn procsub_gt_paren() {
    assert!(has_process_substitution("tee >(wc -l)"));
}

#[test]
fn procsub_false_when_none() {
    assert!(!has_process_substitution("echo hello"));
}

#[test]
fn procsub_inside_single_quotes_ignored() {
    assert!(!has_process_substitution("echo '<(not a sub)'"));
}

// ── has_unquoted ──────────────────────────────────────────────────────────────

#[test]
fn unquoted_finds_literal() {
    assert!(has_unquoted("echo hello world", "hello"));
}

#[test]
fn unquoted_not_in_single_quotes() {
    assert!(!has_unquoted("echo 'hello'", "hello"));
}

#[test]
fn unquoted_not_in_double_quotes() {
    assert!(!has_unquoted(r#"echo "hello""#, "hello"));
}

#[test]
fn unquoted_empty_pattern_always_true() {
    assert!(has_unquoted("anything", ""));
}
