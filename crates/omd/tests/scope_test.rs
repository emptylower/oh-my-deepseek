use omd::scope::WriteScopeValidator;

#[test]
fn exact_path_match() {
    let v = WriteScopeValidator::new(&["src/main.rs"]);
    assert!(v.is_allowed("src/main.rs"));
    assert!(!v.is_allowed("src/lib.rs"));
}

#[test]
fn glob_pattern_match() {
    let v = WriteScopeValidator::new(&["crates/omd/src/**"]);
    assert!(v.is_allowed("crates/omd/src/scope.rs"));
    assert!(v.is_allowed("crates/omd/src/nested/deep.rs"));
    assert!(!v.is_allowed("crates/tui/src/main.rs"));
}

#[test]
fn multiple_patterns() {
    let v = WriteScopeValidator::new(&["src/**/*.rs", "tests/**"]);
    assert!(v.is_allowed("src/foo/bar.rs"));
    assert!(v.is_allowed("tests/integration.rs"));
    assert!(!v.is_allowed("docs/README.md"));
}

#[test]
fn empty_scope_blocks_all() {
    let v = WriteScopeValidator::new(&[]);
    assert!(!v.is_allowed("anything.rs"));
}

#[test]
fn omd_directory_always_writable_for_fuxi() {
    let v = WriteScopeValidator::new(&[".omd/**"]);
    assert!(v.is_allowed(".omd/plans/refactor.md"));
    assert!(!v.is_allowed("src/main.rs"));
}

#[test]
fn path_traversal_blocked() {
    let v = WriteScopeValidator::new(&["src/**"]);
    assert!(!v.is_allowed("src/../etc/passwd"));
    assert!(!v.is_allowed("src/../../secret"));
}

#[test]
fn symlink_escape_blocked() {
    use tempfile::tempdir;
    use std::os::unix::fs::symlink;

    let workspace = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let secret = outside.path().join("secret.txt");
    std::fs::write(&secret, "sensitive data").unwrap();

    // Create a symlink inside workspace pointing outside
    let link = workspace.path().join("src");
    std::fs::create_dir_all(workspace.path()).unwrap();
    symlink(outside.path(), &link).unwrap();

    let v = WriteScopeValidator::with_workspace(&["src/**"], workspace.path());
    // The symlink resolves outside workspace — should be blocked
    assert!(!v.is_allowed("src/secret.txt"));
}
