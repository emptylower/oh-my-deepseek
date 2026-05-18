# Contributing to OhMyDeepSeek (OMD)

Thank you for your interest in contributing to OhMyDeepSeek! This document provides guidelines and instructions for contributing to the project.

## Getting Started

### Prerequisites

- **Rust 1.88+** (edition 2024)
- **Cargo** package manager
- **Git**
- **DeepSeek API key** for testing (`DEEPSEEK_API_KEY` env var)

### Setting Up Development Environment

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/YOUR_USERNAME/oh-my-deepseek.git
   cd oh-my-deepseek
   ```

2. Set up your development environment:
   ```bash
   # Install Rust (if needed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup toolchain install stable
   rustup component add rustfmt clippy
   ```

3. Build the project:
   ```bash
   cargo build --release
   ```

4. Run tests:
   ```bash
   cargo test --workspace
   ```

5. Run OMD specifically:
   ```bash
   cargo test -p omd
   cargo run --bin deepseek-tui -- --help
   ```

## Development Workflow

### Code Style

We follow Rust conventions and enforce consistency across the codebase:

- **Formatting:** Run `cargo fmt` before committing
  ```bash
  cargo fmt --all
  ```

- **Linting:** Run `cargo clippy` and address all warnings
  ```bash
  cargo clippy --workspace --all-targets --all-features
  ```

- **Naming Conventions:**
  - `snake_case` for functions, variables, and module names
  - `CamelCase` for types, structs, traits, and enums
  - `SCREAMING_SNAKE_CASE` for constants

- **Documentation:**
  - Public APIs must have doc comments (`///`)
  - Examples and panics should be documented
  - Use `///` for item docs, `//!` for module docs

Example:
```rust
/// Validates write scope against glob patterns.
///
/// # Arguments
/// * `path` - The file path to validate
/// * `scope` - Glob patterns (e.g., `["src/**/*.rs"]`)
///
/// # Returns
/// `Ok(())` if path is within scope, `Err(ScopeError)` otherwise.
pub fn validate_write_scope(path: &Path, scope: &[String]) -> Result<()> {
    // implementation
}
```

### Testing

OMD ships with **256 tests**. Testing is mandatory for all contributions:

- **Write tests** for new functionality
- **Colocate unit tests** beside the code they cover using `#[cfg(test)]` modules
- **Add integration tests** under the owning crate's `tests/` directory (e.g., `crates/omd/tests/`)
- **Ensure all tests pass** before submitting:
  ```bash
  cargo test --workspace --all-features
  ```

Example test structure:
```rust
// In crates/omd/src/scope.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_write_scope_allows_matching_paths() {
        let scope = vec!["src/**/*.rs".to_string()];
        assert!(validate_write_scope(Path::new("src/main.rs"), &scope).is_ok());
    }

    #[test]
    fn test_validate_write_scope_blocks_traversal() {
        let scope = vec!["src/**/*.rs".to_string()];
        assert!(validate_write_scope(Path::new("../../../etc/passwd"), &scope).is_err());
    }
}
```

Test by crate:
```bash
# OMD core logic
cargo test -p omd

# TUI integration
cargo test -p deepseek-tui

# Full workspace
cargo test --workspace
```

### Commit Messages

Use clear, descriptive commit messages following [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New feature (agent, worker, evidence type, etc.)
- `fix:` Bug fix (FSM guard, scope validation, policy enforcement, etc.)
- `docs:` Documentation changes (README, code comments, guides)
- `refactor:` Code refactoring (no behavior change)
- `test:` Adding or updating tests
- `chore:` Maintenance tasks (deps, CI, tooling)
- `perf:` Performance improvements

Format:
```
<type>(<scope>): <subject>

<body>

<footer>
```

Examples:
```
feat(fsm): add ExplicitSkip evidence type for user-gated transitions

test(scope): add 12 new path-traversal blocking tests

fix(shell_policy): handle quoted process substitution in ReadOnly mode

docs(contributing): clarify test structure for FSM modules
```

When a commit harvests code from a community PR, include:
```
Harvested from PR #N by @author
```

This signals that the PR is closed and credited automatically.

## How Your Contribution Lands

We follow a deliberate "land what's useful, credit the contributor" model:

### Path 1 — Direct Merge (Common)

For well-scoped PRs that:
- Pass all CI checks
- Don't touch security boundaries (sandbox policy, auth, publishing)
- Don't conflict with `main`
- Include tests

Outcome: Maintainer merges directly. Your PR is closed with a thank-you.

### Path 2 — Harvest (Also Valuable)

For PRs that are large, mixed-scope, or conflict with `main`:
- Maintainer extracts useful commits/hunks into a new commit on `main`
- Commit message includes `Harvested from PR #N by @your-handle`
- **This is not rejection** — your code landed
- PR is closed with credit
- You're credited in the next release's CHANGELOG

To increase your chances of direct merge:

1. **Keep PRs single-purpose:** One bug fix per PR; one feature per PR.
2. **Rebase onto current `main`** before opening and after feedback.
3. **Include tests** for new behavior.
4. **Avoid security boundaries** without prior discussion (sandbox policy, auth flows, publishing plumbing).

## Project Structure

OhMyDeepSeek is a Rust workspace with modular crates:

```
crates/
├── omd/              Core OMD logic (agents, FSMs, workers, evidence, policies)
│   ├── src/
│   │   ├── fsm.rs                Agent FSM definitions
│   │   ├── runtime.rs            Engine, delegation, task spawning
│   │   ├── workers.rs            7 specialist workers
│   │   ├── tasks.rs              DAG task graph
│   │   ├── evidence.rs           5-type evidence system
│   │   ├── shell_parser.rs       Command parsing
│   │   ├── shell_policy.rs       Shell policy enforcement
│   │   ├── scope.rs              Write scope validation
│   │   ├── policy.rs             Tool registry guards
│   │   ├── state.rs              State persistence, crash recovery
│   │   ├── transition_guards.rs  Evidence-driven transitions
│   │   ├── prompts/              Agent system prompts
│   │   └── lib.rs
│   └── tests/                    Integration tests (256 tests total)
├── tui/              TUI integration (ratatui, engine hooks)
├── cli/              CLI dispatcher
├── core/             Agent loop, session management
├── state/            SQLite persistence
├── tools/            Tool registry
└── ...other crates
```

### Key Modules in `crates/omd/src/`

| Module | Purpose | Test Count |
|--------|---------|-----------|
| `fsm.rs` | State machines for agents | 65+ |
| `scope.rs` | Write validation, glob matching | 40+ |
| `policy.rs` | Tool enforcement per phase | 50+ |
| `evidence.rs` | Evidence collection & validation | 35+ |
| `state.rs` | Crash recovery, event replay | 30+ |
| `shell_parser.rs` | Command parsing & validation | 36+ |
| Integration suite | End-to-end workflows | 36+ |

### Architecture Overview

```
User Input (TUI)
     ↓
Hongjun (Router)
     ↓ (routes to)
   ┌─┴─┐
   ↓   ↓
Fuxi   Tongtian
(Plan) (Execute)
   ↓   ↓
   └─┬─┘
     ↓ (handoff)
   Pangu (Conductor)
     ↓ (delegates to)
   ┌──────┬──────┬──────┐
   ↓      ↓      ↓      ↓ ... (7 workers)
Jr.   Kunpeng Nuwa  Shennong ...
(Impl) (Read)  (Test)(Write)

State: ~/.omd/sessions/{id}/
  ├── current.json     (FSM state)
  ├── events.jsonl    (event log)
  └── plan.json       (if from Fuxi)
```

## Submitting Changes

### 1. Create a Feature Branch

```bash
git checkout -b feat/your-feature
# or
git checkout -b fix/bug-description
```

### 2. Make Changes & Commit

Follow code style and commit message guidelines:

```bash
cargo fmt --all
cargo clippy --workspace
git add <files>
git commit -m "feat(fsm): add new transition guard"
```

### 3. Ensure CI Passes

```bash
# Format check
cargo fmt --check --all

# Linting
cargo clippy --workspace --all-targets --all-features 2>&1 | head -50

# Type check
cargo check --workspace

# Tests
cargo test --workspace --all-features
```

### 4. Push & Create a Pull Request

```bash
git push origin feat/your-feature
```

Then open a PR on GitHub with:
- Clear title (e.g., "Add ExplicitSkip evidence type")
- Description of what changed and why
- Reference related issues (e.g., "Closes #123")

### 5. Respond to Review Feedback

- Push new commits (don't force-push unless asked)
- Address comments directly in code
- Re-run tests after changes

## Pull Request Guidelines

### Keep PRs Focused

- One bug fix or one feature per PR
- Don't mix refactoring with feature additions
- Don't expand scope mid-review

### Update Documentation

- Update README if adding a new feature
- Add/update code comments for complex logic
- Update CHANGELOG if significant

### Add Tests

- Unit tests alongside code (`#[cfg(test)]`)
- Integration tests in `crates/*/tests/`
- Aim for 80%+ code coverage on new code

### Ensure CI Passes

- All tests pass
- No clippy warnings
- Code is formatted with `cargo fmt`

## Shape of a Typical OMD PR

Well-structured PRs follow a consistent pattern:

**Example 1: New Evidence Type**
- New `ExplicitSkip` variant in `evidence.rs`
- Evidence collection in relevant agent FSM
- Transition guard in `transition_guards.rs`
- Tests in `evidence.rs` + integration test
- CHANGELOG entry under "Added"

Files changed: 3 new, 4 modified, 8+ tests

**Example 2: Worker Enhancement**
- New `Cangjie` worker variant in `workers.rs`
- Write scope definition in `scope.rs`
- Tool registry update in `policy.rs`
- Tests in worker spec + integration
- CHANGELOG entry

Files changed: 3 modified, 15+ tests

**Example 3: Bug Fix**
- Fix to shell policy in `shell_policy.rs`
- Regression test demonstrating the bug
- CHANGELOG entry under "Fixed"

Files changed: 1 modified, 1+ test

Before submitting:

```bash
# Full pre-flight check
cargo fmt --check --all
cargo clippy --workspace --all-targets --all-features 2>&1 | head -50
cargo check --workspace
cargo test --workspace --all-features
```

## Reporting Issues

When reporting bugs, please include:

- **OS and version** (macOS 12.5, Ubuntu 22.04, etc.)
- **Rust version** (`rustc --version`, `cargo --version`)
- **OMD version** (check `.omd-base-version` in repo root)
- **Steps to reproduce** (exact commands)
- **Expected vs. actual behavior**
- **Relevant error logs** (use code blocks)
- **Session state** (if applicable, contents of `~/.omd/sessions/*/current.json`)

Example issue:

```
**Title:** Pangu crashes on cyclic task dependencies

**Steps:**
1. Run `deepseek-omd`
2. Enter plan with circular task dependencies (A→B→C→A)
3. Switch to Pangu executor

**Expected:** Cycle detection error

**Actual:** Panic at runtime::decompose:451

**Error log:**
```
thread 'main' panicked at 'cycle in task graph', crates/omd/src/tasks.rs:451:13
```

**Environment:**
- macOS 13.2, Rust 1.88, OMD at commit abc123
```

## Code of Conduct

Be respectful and inclusive. We welcome contributors of all backgrounds and experience levels.

Discriminatory behavior, harassment, or bad faith will not be tolerated. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

By contributing to OhMyDeepSeek, you agree that your contributions will be licensed under the MIT License.

All contributions are considered under the terms of the repository's LICENSE file.

## Questions?

- Open an issue for bug reports
- Start a discussion for questions or design ideas
- Ask in issues before starting large refactors

Thank you for contributing! 🙏

