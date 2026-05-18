# Changelog

All notable changes to OhMyDeepSeek (OMD) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- Extended specialist workers (code reviewer, architect advisor)
- Git history and blame analysis
- Adaptive retry strategies with backoff tuning
- Cross-session knowledge sharing via `.omd/memory/`
- Shell command auto-completion for policy-safe commands
- Evidence visualization dashboard
- Parallel task execution (beyond max 1 active)

## [0.1.0] - 2026-05-18

Pre-release version. Core OMD system complete and hardened.

### Added

#### Phase 1: Core FSM & Tool Enforcement

- **4 Orchestrator Agents:**
  - Hongjun (鸿钧) — Router: Short-lived intent classifier and session detector
  - Fuxi (伏羲) — Strategist: Read-only interview→explore→architect→plan FSM
  - Pangu (盘古) — Conductor: DAG task decomposition and worker delegation
  - Tongtian (通天) — Solo Executor: Full-autonomy executor for direct tasks

- **7 Specialist Workers:**
  - Tongtian Jr. (通天弟子) — Implementation
  - Kunpeng (鲲鹏) — Code reader/analyst
  - Nuwa (女娲) — Test verifier
  - Shennong (神农) — Test writer
  - Yangmei (杨梅) — File explorer
  - Cangjie (仓颉) — Doc writer
  - Zhurong (祝融) — Debugger

- **Phase FSMs with State Machines:**
  - Fuxi: Interview → Explore → Architect → Plan → Done
  - Pangu: LoadPlan → Decompose → Delegate → Verify → Done
  - Tongtian: Explore → Execute → Verify → Done
  - Per-agent tool filtering and phase-scoped access

- **Tool Registry & Per-Call Enforcement:**
  - Registry-level tool visibility (model sees only allowed tools per phase)
  - Per-call guards (validate phase, scope, policy before execution)
  - Two-layer enforcement: catalog + execution guard

- **Write Scope Validation:**
  - Glob-pattern write scope per worker
  - Path traversal blocking (`..` detection)
  - Symlink escape detection
  - Scope expansion via `NEEDS_SCOPE` protocol error messages
  - Workspace boundary enforcement

#### Phase 2: State Persistence & Hardening

- **Crash Recovery & Event Replay:**
  - `events.jsonl` as source of truth (append-only event log)
  - Full state reconstruction on startup via event replay
  - Session isolation via lock files (prevents concurrent access)
  - Per-session state: `~/.omd/sessions/{session_id}/`

- **Evidence System (5 Types):**
  - `FileDiscovery`: Files read, analyzed, modified
  - `TestResult`: Test pass/fail with logs and exit codes
  - `GitDiff`: Staged/unstaged diffs in numstat format
  - `PlanArtifact`: Generated plan JSON (Fuxi output)
  - `ExplicitSkip`: Opt-out gates (user-gated transitions)
  - Client-verified evidence before transitions

- **Task Graph & DAG Execution:**
  - DAG decomposition of plans into typed task nodes
  - Dependency tracking between tasks
  - Blocked-task auto-management (retries, fallbacks)
  - Max 1 active task at a time
  - Task-level retry counting with exponential backoff

- **Shell Command Parsing & Policy:**
  - Full shell parser (quotes, escapes, process substitution, redirection)
  - 3-tier shell policy: None / ReadOnly / Full
  - Command validation before execution
  - Quote matching and escape sequence handling
  - Subshell and command substitution detection

- **Fuxi Handoff Widget:**
  - One-key confirm prompt after plan generation
  - UI integration with TUI (user switches to Pangu explicitly)
  - Plan artifact inspection before execution

- **Test Coverage:**
  - 256 unit + integration tests
  - FSM transition validation (65+ tests)
  - Write scope and glob matching (40+ tests)
  - Tool policy enforcement (50+ tests)
  - Evidence system (35+ tests)
  - State persistence and replay (30+ tests)
  - End-to-end workflows (36+ tests)

### Changed

- DeepSeek-TUI Agent/Plan/YOLO modes replaced with OMD routing
- Tool access now phase-gated per agent
- Session state stored in `~/.omd/sessions/` instead of DeepSeek-TUI's default location

### Fixed

- Tool calls validated before execution (prevents unauthorized writes)
- Path traversal attacks blocked by scope validation
- Symlink escapes detected and rejected
- Shell injection mitigated via command parsing
- Concurrent session access prevented by lock files

---

## Release Signatures

All OMD releases are tagged with the date of release.

- **v0.1.0**: 2026-05-18 — Initial pre-release, core OMD complete
- **Base DeepSeek-TUI**: v0.8.36 (2026-05-14)

---

## Upgrade Path

OhMyDeepSeek v0.1.0 is pre-release and API-stable for the core agents and FSMs. Future minor versions (0.2.x, 0.3.x) will add new worker types and features without breaking existing plans or session format.

Breaking changes (if any) will bump to 1.0.0 and be documented here with migration guidance.

---

## Compatibility

- **Rust:** 1.88+ (edition 2024)
- **DeepSeek API:** v1 (compatible with `deepseek-v4-pro`, `deepseek-v4-flash`)
- **DeepSeek-TUI:** v0.8.36+
- **OS:** macOS 12+, Linux (glibc 2.31+)
- **Shell:** bash, zsh, sh

---

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

When harvesting community contributions, commits include:
```
Harvested from PR #N by @author
```

Credit is given in the next release's changelog.

