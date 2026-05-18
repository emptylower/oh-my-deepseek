# OhMyDeepSeek (OMD)

<div align="center">

```
 ___  _   _           _ _      ____        _
/ _ \| | | |_ __ ___| _  |   |  _ \  ___ | |_ _ __
| | | | |_| | '_ \/ _ \ || |_  | | |_/ _ \| __| '_ \
| | | |  _  | | | |  __/\ || ' \ | | | (_) | |_| | | |
|_| |_|_| |_|_| |_|___| _||_||_| |_| \___/ \__|_| |_|
                                                  Seeker
```

**万法归宗** — Multi-Agent Orchestration for DeepSeek-TUI

A purpose-built multi-agent orchestration system that replaces DeepSeek-TUI's native Agent/Plan/YOLO modes with 4 specialized orchestrator agents, each with internal FSMs, tool guardrails, and evidence-driven transitions.

[![GitHub](https://img.shields.io/badge/GitHub-emptylower%2Foh--my--deepseek-181717?logo=github)](https://github.com/emptylower/oh-my-deepseek)
[![License](https://img.shields.io/badge/License-MIT-yellow)](https://github.com/emptylower/oh-my-deepseek?tab=MIT-1-ov-file)
[![Rust](https://img.shields.io/badge/Rust-1.88+-CE422B?logo=rust)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-256%20passing-brightgreen)](#project-structure)
[![Version](https://img.shields.io/badge/Version-0.1.0-blue)](https://github.com/emptylower/oh-my-deepseek/releases)

</div>

---

## Why OhMyDeepSeek?

DeepSeek-TUI's built-in Agent/Plan/YOLO modes treat all tasks the same way. OhMyDeepSeek replaces them with **4 purpose-built orchestrator agents**, each specialized for a different phase of work:

- **Short-lived routers** that classify intent and detect stale sessions
- **Read-only strategists** that interview, explore, architect, and plan — then hand off with a one-key confirm
- **DAG-aware conductors** that decompose plans into task graphs and delegate to specialist workers
- **Full-autonomy executors** for direct tasks that don't need planning

Every agent enforces a client-side FSM, writes evidence to disk, and gates tool access per phase. The system recovers from crashes by replaying events. Write scope is validated per worker. Shell commands are parsed and policy-checked. Plans are composable and resumable.

Inspired by [OpenAgent](https://github.com/openagentinc/openagent)'s approach to [OpenCode](https://github.com/openagentinc/opencoder).

---

## The 4 Orchestrator Agents

### 鸿钧 — Hongjun (Router)
**"万法归宗"** — Classify intent and route to the right agent.

- **Lifetime:** 1–2 turns. Short-lived router.
- **Tools:** Model call only (no file/shell access).
- **Mission:** Detect unfinished sessions, classify task intent, route to Strategist or Solo Executor.
- **Output:** Typed decision: `RouteToStrategist`, `RouteToSoloExecutor`, or `ResumeSession`.

### 伏羲 — Fuxi (Strategist)
**"先知先觉"** — Interview, explore, architect, plan. Read-only + `.omd/` writes.

- **Lifetime:** Multi-turn interview → structured plan.
- **FSM:** `Interview` → `Explore` → `Architect` → `Plan` → `Done`.
- **Tools:** File read, web search, codebase traversal (no shell execution, no edits).
- **Write Scope:** Plan files to `.omd/plans/` + session state.
- **Output:** Structured plan artifact, then one-key confirm widget. User switches to Pangu.
- **Evidence:** `FileDiscovery`, `PlanArtifact`, `ExplicitSkip`.

### 盘古 — Pangu (Conductor)
**"开天辟地"** — Decompose plans into DAG task graphs, delegate to 7 specialist workers.

- **Lifetime:** Loads plan, decomposes into tasks, delegates, verifies completion.
- **FSM:** `LoadPlan` → `Decompose` → `Delegate` → `Verify` → `Done`.
- **Task Graph:** DAG with dependency tracking, blocked-task auto-management, max 1 active task.
- **Output:** Task graph, delegation to workers, final verification.
- **Evidence:** `GitDiff`, `TestResult`, `FileDiscovery`.

### 通天 — Tongtian (Solo Executor)
**"无所不能"** — Full autonomous agent for direct, unbounded tasks.

- **Lifetime:** Explore, execute, verify. Full tool access.
- **FSM:** `Explore` → `Execute` → `Verify` → `Done`.
- **Tools:** All tools (files, shell, git, web search, etc.).
- **Use Case:** Direct `/fix <issue>`, `/implement <feature>`, one-shot tasks.
- **Evidence:** `GitDiff`, `TestResult`, `FileDiscovery`.

---

## 7 Specialist Workers (Spawned by Pangu)

| Worker | Chinese | Role | Can Write |
|--------|---------|------|-----------|
| **Tongtian Jr.** | 通天弟子 | Implementation | Yes |
| **Kunpeng** | 鲲鹏 | Code reader/analyst | No |
| **Nuwa** | 女娲 | Test verifier | No |
| **Shennong** | 神农 | Test writer | Yes |
| **Yangmei** | 杨梅 | File explorer | No |
| **Cangjie** | 仓颉 | Doc writer | Yes |
| **Zhurong** | 祝融 | Debugger | Yes |

Each worker has:
- **Scoped write access** (glob patterns, no path traversal, symlink escape detection)
- **Tool registry** (only the tools for their role)
- **Evidence collection** (before transitions)
- **Retry counting** (auto-managed by Pangu)

---

## Quick Start

### Install from Source (Recommended)

```bash
git clone https://github.com/emptylower/oh-my-deepseek.git
cd oh-my-deepseek

# Build the release binary
cargo build --release --bin deepseek-tui

# Install to ~/.local/bin
mkdir -p ~/.local/bin
cp target/release/deepseek-tui ~/.local/bin/deepseek-omd
chmod +x ~/.local/bin/deepseek-omd

# Verify PATH contains ~/.local/bin, then:
deepseek-omd
```

### Use the Install Script

```bash
bash scripts/omd-install.sh
```

Installs to `~/.local/bin/deepseek-omd` and uses `~/.deepseek-omd` for config.

### Requirements

- **Rust 1.88+** (edition 2024)
- **DeepSeek API key** (`DEEPSEEK_API_KEY` env var)
- **macOS or Linux** (Windows untested)

### Uninstall

```bash
deepseek-omd --uninstall
```

---

## Core Architecture

```
crates/omd/           Core OMD logic
├── fsm.rs            Agent FSM state machines
├── runtime.rs        Runtime engine, delegation, task spawning
├── workers.rs        7 specialist worker definitions
├── tasks.rs          DAG task graph, dependency tracking
├── evidence.rs       5-type evidence system (FileDiscovery, TestResult, GitDiff, PlanArtifact, ExplicitSkip)
├── shell_parser.rs   Shell command parser (quotes, escapes, process substitution)
├── shell_policy.rs   Shell policy tiers (None/ReadOnly/Full)
├── scope.rs          Write scope validation, path traversal blocking
├── policy.rs         Tool registry enforcement, per-phase guards
├── state.rs          Runtime state, crash recovery, events.jsonl
├── transition_guards.rs  Evidence-driven state transitions
├── prompts/          Agent-specific system prompts
└── tests/            256 unit + integration tests (state machine coverage, policy validation)

crates/tui/          TUI integration, engine hooks, UI widgets
├── commands/        Command dispatch (Agent/Plan/YOLO → OMD routing)
├── ui/              Fuxi handoff widget (one-key confirm)
└── engine/          Hook registration

.omd/sessions/       Runtime state (current.json + events.jsonl per session)
.omd/plans/          Fuxi-generated plan files (JSON artifacts)
```

### Two-Layer Tool Enforcement

1. **Registry:** Model sees the catalog; tools are filtered per agent/phase.
2. **Per-Call Guard:** Before execution, each tool call is validated:
   - Phase check (is this tool allowed now?)
   - Write scope check (glob patterns, no escapes)
   - Shell policy check (policy tier + parsed command)

### State Persistence & Crash Recovery

- **Single source of truth:** `events.jsonl` (append-only event log)
- **On startup:** Full replay of events → in-memory state
- **Sessions:** `~/.omd/sessions/{session_id}/` contains `current.json` + `events.jsonl`
- **Lock files:** Prevent concurrent session access

### Task Graph & DAG Execution

- **Decomposition:** Plans become typed task nodes with dependencies
- **Blocked tasks:** Auto-managed by conductor (retries, fallbacks)
- **Max 1 active:** Only one task runs concurrently; others wait
- **Retry counting:** Task-level retry tracking with exponential backoff

---

## Key Features

### Phase FSM Enforcement

Each agent has an internal FSM. Transitions require evidence:

```
Fuxi:    Interview → Explore → Architect → Plan → Done
Pangu:   LoadPlan → Decompose → Delegate → Verify → Done
Tongtian: Explore → Execute → Verify → Done
```

Tools are scoped to phases. Moving to the next phase requires evidence (FileDiscovery, TestResult, etc.) that the previous phase completed.

### Write Scope Validation

Workers declare write scope as glob patterns:

```
Tongtian Jr: ["src/**/*.rs", "tests/**/*.rs"]
Shennong:    ["tests/**/*.rs"]
Cangjie:     ["docs/**/*.md", "README.md"]
Zhurong:     ["*.log", ".debug/**/*"]
```

Before a write, the scope is validated:
- Path is inside the workspace
- Path matches declared globs
- No symlink escapes
- No `..` traversals outside scope

### Shell Policy Tiers

Three levels of shell access:

- **None:** No shell execution (read-only agents)
- **ReadOnly:** File reads + queries only (e.g., `find`, `grep`, `git log`)
- **Full:** Arbitrary commands (implementers, debuggers)

Policy is enforced by parsing commands before execution (quotes, escapes, process substitution are validated).

### Evidence System (5 Types)

Transitions are gated by evidence. Workers emit:

1. **FileDiscovery:** Files read, analyzed, or modified
2. **TestResult:** Test pass/fail with logs
3. **GitDiff:** Staged/unstaged diffs (numstat format)
4. **PlanArtifact:** Generated plan (JSON)
5. **ExplicitSkip:** Opt-out (user gates certain transitions)

Evidence is stored, replayed on crash, and verified before phase transitions.

### Fuxi Handoff Widget

After Fuxi generates a plan, the TUI displays a one-key confirm prompt:

```
┌─ Fuxi Generated Plan ─────────────────────┐
│ Strategy: Interview 3 files, architect DB │
│ Plan:     Implement 8-task DAG            │
├───────────────────────────────────────────┤
│ [Y] Switch to Pangu Executor              │
│ [N] Revise in Fuxi                        │
└───────────────────────────────────────────┘
```

---

## Project Phases & Milestones

### Phase 1: Core FSM & Tool Enforcement
- Agent FSMs with phase gating
- Tool registry + per-call guard enforcement
- Write scope validation and glob patterns
- Shell policy parser and enforcement

### Phase 2: State Persistence & Hardening
- Event-driven crash recovery (`events.jsonl`)
- Lock files for concurrent session safety
- Evidence system (5 types, client-verified)
- Full test coverage (256 tests across FSM, scope, policy)
- Fuxi handoff widget integration
- Shell parser for complex commands
- Codex reviewed

### Phase 3: Expansion & Polish (Planned)
- Extended specialist workers (code reviewer, architect)
- Git history analysis
- Adaptive retry strategies
- Cross-session knowledge sharing

---

## Configuration

Configuration lives in `~/.deepseek-omd/config.toml`. Basic example:

```toml
[omd]
# Default agent routing
default_agent = "auto"  # auto, strategist, executor, solo

# Session persistence
session_dir = "~/.omd/sessions"
plan_dir = "~/.omd/plans"

# FSM & evidence
enforce_transitions = true
require_evidence = true

# Shell policy
shell_policy = "full"  # none, readonly, full
parse_shell_commands = true

# Workers
max_parallel_tasks = 1
worker_timeout_secs = 600
```

---

## Testing

OMD ships with **256 tests**:

```bash
# Run all tests
cargo test --workspace

# Run only OMD tests
cargo test -p omd

# Test with output
cargo test -- --nocapture --test-threads=1
```

Test categories:

- **FSM tests** (65+): State machine transitions, phase guards
- **Scope tests** (40+): Write validation, path traversal, glob matching
- **Policy tests** (50+): Tool enforcement, shell policy
- **Evidence tests** (35+): Evidence collection, transitions
- **State tests** (30+): Crash recovery, event replay
- **Integration tests** (36+): End-to-end workflows

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Development setup
- Code style and testing requirements
- Commit message format
- PR guidelines
- How contributions land (direct merge vs. harvest)

Key points:

- Rust 1.88+, `cargo fmt`, `cargo clippy`
- Tests colocate with code (`#[cfg(test)]` modules)
- Integration tests under `crates/*/tests/`
- Single-purpose PRs land faster

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history. Current stable: **0.8.36** (DeepSeek-TUI base). OMD version: **0.1.0** (pre-release).

---

## License

MIT. See [LICENSE](LICENSE) for details.

---

## Acknowledgments

- Built on [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) v0.8.36 (fork)
- Inspired by [OpenAgent](https://github.com/openagentinc/openagent) and OpenCode
- Original author: **emptylower**
- DeepSeek API: [DeepSeek AI](https://www.deepseek.com/)

---

## Resources

- **GitHub:** https://github.com/emptylower/oh-my-deepseek
- **DeepSeek API:** https://platform.deepseek.com/
- **DeepSeek-TUI:** https://github.com/Hmbown/DeepSeek-TUI
- **Rust:** https://www.rust-lang.org/

