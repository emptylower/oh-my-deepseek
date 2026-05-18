# OhMyDeepSeek (OMD)

<div align="center">

**Multi-Agent Orchestration for DeepSeek-TUI**

Replace DeepSeek-TUI's native Agent/Plan/YOLO modes with 4 purpose-built orchestrator agents,
each with internal FSMs, tool guardrails, and evidence-driven transitions.

[![GitHub](https://img.shields.io/badge/GitHub-emptylower%2Foh--my--deepseek-181717?logo=github)](https://github.com/emptylower/oh-my-deepseek)
[![License](https://img.shields.io/badge/License-MIT-yellow)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.88+-CE422B?logo=rust)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-256%20passing-brightgreen)](#testing)

[中文](README.md)

</div>

---

## Why OhMyDeepSeek?

DeepSeek-TUI's built-in Agent/Plan/YOLO modes treat all tasks the same. OhMyDeepSeek replaces them with **4 specialized orchestrator agents**:

- **Short-lived routers** that classify intent and detect stale sessions
- **Read-only strategists** that interview, explore, architect, and plan — then hand off with a one-key confirm
- **DAG-aware conductors** that decompose plans into task graphs and delegate to 7 specialist workers
- **Full-autonomy executors** for direct tasks that don't need planning

Every agent enforces a client-side FSM, writes evidence to disk, and gates tool access per phase. The system recovers from crashes by replaying events. Write scope is glob-validated per worker. Shell commands are parsed and policy-checked.

> Inspired by [OpenAgent](https://github.com/openagentinc/openagent)'s approach to [OpenCode](https://github.com/openagentinc/opencoder)

---

## The 4 Orchestrator Agents

### Hongjun (Router)

> Classify intent, route to the right agent.

| Attribute | Description |
|-----------|-------------|
| Lifecycle | 1-2 turns, short-lived |
| Tools | Model call only (no file/shell access) |
| Mission | Detect unfinished sessions, classify task intent, route to Strategist or Executor |
| Output | `RouteToStrategist` / `RouteToSoloExecutor` / `ResumeSession` |

### Fuxi (Strategist)

> Interview, explore, architect, plan. Read-only + `.omd/` writes.

| Attribute | Description |
|-----------|-------------|
| FSM | `Interview` → `Explore` → `Architect` → `Plan` → `Done` |
| Tools | Read-only + `.omd/` directory writes |
| Write Scope | Plan files to `.omd/plans/` + session state |
| Output | Structured plan → TUI one-key confirm widget → switch to Pangu |
| Evidence | `FileDiscovery`, `PlanArtifact`, `ExplicitSkip` |

### Pangu (Conductor)

> Decompose plans into DAG task graphs, delegate to 7 specialist workers.

| Attribute | Description |
|-----------|-------------|
| FSM | `LoadPlan` → `Decompose` → `Delegate` → `Verify` → `Done` |
| Task Graph | DAG with dependency tracking, blocked auto-management, max 1 active task |
| Delegation | 7 specialist workers, assigned by task type |
| Guard | Delegate→Verify requires all tasks returned |
| Evidence | `GitDiff`, `TestResult`, `FileDiscovery` |

### Tongtian (Solo Executor)

> Full autonomy for direct tasks.

| Attribute | Description |
|-----------|-------------|
| FSM | `Explore` → `Execute` → `Verify` → `Done` |
| Tools | All tools in Execute phase |
| Use Case | Direct bug fixes, feature implementation, one-shot tasks |
| Evidence | `GitDiff`, `TestResult`, `FileDiscovery` |

---

## 7 Specialist Workers

Spawned by Pangu during the Delegate phase:

| Worker | Role | Can Write |
|--------|------|:---------:|
| **Tongtian Jr.** | Implementation | Yes |
| **Kunpeng** | Code analysis | No |
| **Nuwa** | Test verification | No |
| **Shennong** | Test writing | Yes |
| **Yangmei** | File exploration | No |
| **Cangjie** | Documentation | Yes |
| **Zhurong** | Debugging | Yes |

Each worker has:
- **Scoped write access** — glob patterns, path traversal blocking, symlink escape detection
- **Role-specific tool registry** — only the tools needed for their job
- **Evidence collection** — required before phase transitions
- **Retry counting** — auto-managed by Pangu, failed tasks routed to Zhurong

---

## Quick Start

### Requirements

- **Rust 1.88+** (edition 2024)
- **DeepSeek API Key** (`DEEPSEEK_API_KEY` env var)
- **macOS / Linux** (Windows untested)

### Install from Source

```bash
git clone https://github.com/emptylower/oh-my-deepseek.git
cd oh-my-deepseek

# Build
cargo build --release --bin deepseek-tui

# Install to PATH
mkdir -p ~/.local/bin
cp target/release/deepseek-tui ~/.local/bin/deepseek-omd
chmod +x ~/.local/bin/deepseek-omd

# Launch
deepseek-omd
```

### Install Script

```bash
bash scripts/omd-install.sh
```

### Uninstall

```bash
deepseek-omd --uninstall
```

> [!NOTE]
> When running as `deepseek-omd`, Tab cycles only the 4 OMD agents (Hongjun → Tongtian → Fuxi → Pangu). Native Agent/Plan/YOLO modes are hidden.

---

## Typical Workflow

```
User → Launch deepseek-omd
         │
    Hongjun (Router)
    "Analyzing your request..."
         │
    ┌────┴────┐
    ▼         ▼
  Fuxi     Tongtian
 (Plan)    (Direct)
    │
  Interview → Explore → Architect → Plan → Done
    │
  ┌─┴─ One-key handoff ─┐
  ▼                      │
Pangu (Execute)          │
  │                      │
  LoadPlan → Decompose → Delegate → Verify → Done
                     │
              ┌──────┼──────┬──────┐
              ▼      ▼      ▼      ▼
         Tongtian  Kunpeng  Nuwa  Shennong ...
           Jr.   (Analyze) (Test) (Write)
         (Build)
```

---

## Core Architecture

```
crates/omd/                 Core OMD logic
├── fsm.rs                  Agent FSMs
├── runtime.rs              Runtime engine, delegation, task spawning
├── workers.rs              7 specialist worker definitions
├── tasks.rs                DAG task graph, dependency tracking
├── evidence.rs             5-type evidence system
├── shell_parser.rs         Shell command parser (598 lines)
├── shell_policy.rs         Shell policy tiers (None/ReadOnly/Full)
├── scope.rs                Write scope validation
├── policy.rs               Tool registry, per-phase guards
├── state.rs                State persistence, crash recovery
└── transition_guards.rs    Evidence-driven transition guards

crates/tui/                 TUI integration
├── commands/omd.rs         /omd-execute, /omd-phase-complete commands
├── tools/omd.rs            Native OMD tools
├── tools/omd_delegate.rs   Pangu delegation tool
├── core/engine.rs          Engine hooks, runtime init
└── tui/ui.rs               Fuxi handoff widget

.omd/sessions/              Runtime state (current.json + events.jsonl)
.omd/plans/                 Fuxi-generated plan artifacts
```

---

## Key Technical Features

### Two-Layer Tool Enforcement

1. **Registry** — model sees only the tool catalog allowed for the current phase
2. **Per-call guard** — every tool call is validated: phase permission + write scope + shell policy

### Typed Evidence System

Phase transitions are gated by 5 evidence types:

| Type | Description | Verification |
|------|-------------|--------------|
| `FileDiscovery` | File existence | Filesystem check |
| `TestResult` | Test pass/fail | Shell audit log match |
| `GitDiff` | File change stats | `git diff --numstat` + untracked detection |
| `PlanArtifact` | Plan file | File exists + checkbox format |
| `ExplicitSkip` | User bypass | User-only, model cannot self-approve |

### Shell Parser

Full argv tokenizer (598 lines) handling:
- Single/double quotes, backslash escapes
- Chain operators `&&` `||` `;` `&`
- Pipes `|`, redirects `>` `>>`
- Command substitution `$(...)` and backticks (including inside double quotes)
- Process substitution `<(...)` `>(...)`

### Crash Recovery

- **Source of truth** — `events.jsonl` (append-only event log)
- **On startup** — full event replay, rebuild phase + task graph
- **Lock files** — PID-based liveness check, prevent concurrent sessions
- **Metadata merge** — events control status, `current.json` supplements rich metadata

### Stall Handling

- **Auto-hint** — prompt after model turn ends without phase transition
- **Escape hatch** — `/omd-phase-complete <target>` skips evidence (keeps FSM guards)

---

## Testing

OMD ships with **256 tests**:

```bash
# Run all OMD tests
cargo test -p omd

# Run specific modules
cargo test -p omd shell_parser    # Shell parser (101 tests)
cargo test -p omd evidence        # Evidence verification
cargo test -p omd transition      # Transition guards
cargo test -p omd golden          # Tool whitelist golden tests
cargo test -p omd tasks           # Task graph
cargo test -p omd runtime         # Runtime + crash recovery
```

---

## Development Milestones

| Phase | Content | Status |
|-------|---------|:------:|
| Phase 1 (Plan 1-2) | Core FSM, 4 agents, 7 workers, tool enforcement, write scope, shell policy | Done |
| Phase 2 (Plan 3) | State persistence, crash recovery, evidence system, install scripts | Done |
| Phase 2 (Plan 4) | Hardening — lock files, transition guards, golden tests, Codex reviewed | Done |
| Phase 2 (Plan 5) | Spec complete — parser, GitDiff, user-gating, event replay, handoff widget | Done |
| Phase 3 (Planned) | Extended workers, git history analysis, adaptive retry, cross-session knowledge | Planned |

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Acknowledgments

- Built on [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) v0.8.36 (fork)
- Inspired by [OpenAgent](https://github.com/openagentinc/openagent) and OpenCode
- DeepSeek API: [DeepSeek AI](https://www.deepseek.com/)

## License

[MIT](LICENSE)
