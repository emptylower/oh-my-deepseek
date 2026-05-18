# OhMyDeepSeek (OMD)

<div align="center">

**万法归宗 — DeepSeek-TUI 多智能体编排系统**

用 4 个专用编排智能体替换 DeepSeek-TUI 原生的 Agent/Plan/YOLO 模式。
每个智能体拥有独立的有限状态机、工具权限守卫和证据驱动的阶段转换。

Replace DeepSeek-TUI's native Agent/Plan/YOLO modes with 4 purpose-built orchestrator agents,
each with internal FSMs, tool guardrails, and evidence-driven transitions.

[![GitHub](https://img.shields.io/badge/GitHub-emptylower%2Foh--my--deepseek-181717?logo=github)](https://github.com/emptylower/oh-my-deepseek)
[![License](https://img.shields.io/badge/License-MIT-yellow)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.88+-CE422B?logo=rust)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-256%20passing-brightgreen)](#测试)

</div>

---

## 为什么需要 OhMyDeepSeek？ / Why OhMyDeepSeek?

DeepSeek-TUI 内置的 Agent/Plan/YOLO 模式对所有任务一视同仁。OhMyDeepSeek 用 **4 个专用编排智能体** 替换它们：

DeepSeek-TUI's built-in modes treat all tasks the same. OhMyDeepSeek replaces them with **4 specialized agents**:

- **短生命路由器** — 分类意图、检测未完成会话 / Short-lived routers that classify intent
- **只读策略师** — 访谈→探索→架构→规划，然后一键交接 / Read-only strategists with one-key handoff
- **DAG 指挥官** — 将计划分解为任务图，委派给 7 个专业工人 / DAG conductors delegating to 7 specialist workers
- **全权执行者** — 直接执行不需要规划的任务 / Full-autonomy executors for direct tasks

所有智能体都在客户端强制执行 FSM 状态机、将证据写入磁盘、按阶段控制工具访问权限。系统通过重放事件日志从崩溃中恢复。每个工人的写入范围经过 glob 模式验证。Shell 命令经过解析和策略检查。

Every agent enforces a client-side FSM, writes evidence to disk, and gates tool access per phase. The system recovers from crashes by replaying events. Write scope is glob-validated per worker. Shell commands are parsed and policy-checked.

> 灵感来源 / Inspired by [OpenAgent](https://github.com/openagentinc/openagent) 对 [OpenCode](https://github.com/openagentinc/opencoder) 的改造思路

---

## 四大编排智能体 / The 4 Orchestrator Agents

### 鸿钧 Hongjun — 路由器 / Router

> **万法归宗** — 分类意图，路由到正确的智能体 / Classify intent, route to the right agent

| 属性 / Attr | 说明 / Description |
|---|---|
| 生命周期 | 1-2 轮对话，短命路由 / 1-2 turns, short-lived |
| 工具权限 | 仅模型调用，无文件/Shell 访问 / Model call only |
| 职责 | 检测未完成会话、分类任务意图、路由到策略师或执行者 |
| 输出 | `RouteToStrategist` / `RouteToSoloExecutor` / `ResumeSession` |

### 伏羲 Fuxi — 策略师 / Strategist

> **先知先觉** — 访谈、探索、架构、规划 / Interview, explore, architect, plan

| 属性 / Attr | 说明 / Description |
|---|---|
| FSM | `Interview` → `Explore` → `Architect` → `Plan` → `Done` |
| 工具权限 | 只读 + `.omd/` 目录写入 / Read-only + `.omd/` writes |
| 写入范围 | 计划文件写入 `.omd/plans/` + 会话状态 |
| 输出 | 结构化计划 → TUI 一键确认组件 → 切换到盘古 |
| 证据类型 | `FileDiscovery`, `PlanArtifact`, `ExplicitSkip` |

### 盘古 Pangu — 指挥官 / Conductor

> **开天辟地** — 分解计划为 DAG 任务图，委派 7 个工人 / Decompose plans, delegate to 7 workers

| 属性 / Attr | 说明 / Description |
|---|---|
| FSM | `LoadPlan` → `Decompose` → `Delegate` → `Verify` → `Done` |
| 任务图 | DAG 依赖追踪、阻塞自动管理、最多 1 个活跃任务 |
| 委派 | 7 个专业工人，按任务类型分配 |
| 守卫 | Delegate→Verify 要求所有任务已返回结果 |
| 证据类型 | `GitDiff`, `TestResult`, `FileDiscovery` |

### 通天 Tongtian — 独行侠 / Solo Executor

> **无所不能** — 全自主执行，适合直接任务 / Full autonomy for direct tasks

| 属性 / Attr | 说明 / Description |
|---|---|
| FSM | `Explore` → `Execute` → `Verify` → `Done` |
| 工具权限 | Execute 阶段拥有全部工具 / All tools in Execute phase |
| 适用场景 | 直接修 bug、实现功能、一次性任务 |
| 证据类型 | `GitDiff`, `TestResult`, `FileDiscovery` |

---

## 七大专业工人 / 7 Specialist Workers

由盘古在 Delegate 阶段按需生成 / Spawned by Pangu during Delegate phase:

| 工人 / Worker | 中文名 | 角色 / Role | 可写 / Write |
|---------------|--------|-------------|:------------:|
| **Tongtian Jr.** | 通天弟子 | 代码实现 / Implementation | Yes |
| **Kunpeng** | 鲲鹏 | 代码阅读分析 / Code analysis | No |
| **Nuwa** | 女娲 | 测试验证 / Test verification | No |
| **Shennong** | 神农 | 测试编写 / Test writing | Yes |
| **Yangmei** | 杨梅 | 文件探索 / File exploration | No |
| **Cangjie** | 仓颉 | 文档编写 / Documentation | Yes |
| **Zhurong** | 祝融 | 调试排错 / Debugging | Yes |

每个工人都有 / Each worker has:
- **写入范围约束** — glob 模式、路径穿越阻断、符号链接逃逸检测 / Scoped write access with glob patterns
- **独立工具注册表** — 仅包含角色所需工具 / Role-specific tool registry
- **证据收集** — 阶段转换前必须提交 / Evidence before transitions
- **重试计数** — 盘古自动管理 / Auto-managed retries

---

## 快速开始 / Quick Start

### 环境要求 / Requirements

- **Rust 1.88+** (edition 2024)
- **DeepSeek API Key** (环境变量 `DEEPSEEK_API_KEY`)
- **macOS / Linux** (Windows 未测试)

### 从源码安装 / Install from Source

```bash
git clone https://github.com/emptylower/oh-my-deepseek.git
cd oh-my-deepseek

# 构建 / Build
cargo build --release --bin deepseek-tui

# 安装到 PATH / Install
mkdir -p ~/.local/bin
cp target/release/deepseek-tui ~/.local/bin/deepseek-omd
chmod +x ~/.local/bin/deepseek-omd

# 启动 / Launch
deepseek-omd
```

### 使用安装脚本 / Install Script

```bash
bash scripts/omd-install.sh
```

### 卸载 / Uninstall

```bash
deepseek-omd --uninstall
```

> [!NOTE]
> 作为 `deepseek-omd` 运行时，Tab 键只在 4 个 OMD 智能体间切换（鸿钧→通天→伏羲→盘古），原生的 Agent/Plan/YOLO 模式不可见。
>
> When running as `deepseek-omd`, Tab cycles only OMD agents. Native Agent/Plan/YOLO modes are hidden.

---

## 核心架构 / Core Architecture

```
crates/omd/                 核心 OMD 逻辑 / Core OMD logic
├── fsm.rs                  智能体有限状态机 / Agent FSMs
├── runtime.rs              运行时引擎、委派、任务生成 / Runtime, delegation, spawning
├── workers.rs              7 个专业工人定义 / Worker definitions
├── tasks.rs                DAG 任务图、依赖追踪 / Task graph, dependencies
├── evidence.rs             5 类证据系统 / 5-type evidence system
├── shell_parser.rs         Shell 命令解析器 / Command parser (598 lines)
├── shell_policy.rs         Shell 策略层 / Policy tiers (None/ReadOnly/Full)
├── scope.rs                写入范围验证 / Write scope validation
├── policy.rs               工具注册表、按阶段守卫 / Tool registry, per-phase guards
├── state.rs                状态持久化、崩溃恢复 / State persistence, crash recovery
└── transition_guards.rs    证据驱动的转换守卫 / Evidence-driven guards

crates/tui/                 TUI 集成 / TUI integration
├── commands/omd.rs         /omd-execute, /omd-phase-complete 命令
├── tools/omd.rs            原生 OMD 工具 / Native OMD tools
├── tools/omd_delegate.rs   盘古委派工具 / Pangu delegation
├── core/engine.rs          引擎钩子、运行时初始化 / Engine hooks
└── tui/ui.rs               伏羲交接组件 / Fuxi handoff widget

.omd/sessions/              运行时状态 / Runtime state per session
.omd/plans/                 伏羲生成的计划文件 / Plan artifacts
```

---

## 关键技术特性 / Key Technical Features

### 两层工具执行 / Two-Layer Tool Enforcement

1. **注册表层** — 模型只能看到当前阶段允许的工具 / Model sees only phase-allowed tools
2. **调用守卫层** — 每次工具调用前验证：阶段权限 + 写入范围 + Shell 策略 / Per-call validation

### 类型化证据系统 / Typed Evidence System

阶段转换由证据门控，5 种类型 / Transitions gated by 5 evidence types:

| 证据类型 | 说明 | 验证方式 |
|----------|------|----------|
| `FileDiscovery` | 文件存在性 | 文件系统检查 |
| `TestResult` | 测试通过/失败 | Shell 审计日志匹配 |
| `GitDiff` | 文件变更统计 | `git diff --numstat` + 新文件检测 |
| `PlanArtifact` | 计划文件 | 文件存在 + checkbox 格式 |
| `ExplicitSkip` | 用户跳过 | 仅用户可触发，模型不可自批准 |

### Shell 解析器 / Shell Parser

完整的 argv 分词器（598 行），处理 / Full tokenizer handling:
- 单/双引号、转义字符 / Quotes and escapes
- 链式操作符 `&&` `||` `;` `&` / Chain operators
- 管道 `|`、重定向 `>` `>>` / Pipes and redirects
- 命令替换 `$(...)` 和反引号（含双引号内检测）/ Command substitution (including inside double quotes)
- 进程替换 `<(...)` `>(...)` / Process substitution

### 崩溃恢复 / Crash Recovery

- **真相源** — `events.jsonl`（仅追加事件日志）/ Append-only event log
- **启动时** — 完整事件重放，重建阶段 + 任务图 / Full replay on startup
- **锁文件** — PID 检测，防止并发会话 / PID-based locking
- **元数据合并** — 事件日志控制状态，`current.json` 补充富元数据 / Events for status, snapshot for metadata

### 卡顿处理 / Stall Handling

- **提示** — 模型回合结束后自动提示当前阶段 / Auto-hint after model turn
- **逃生舱** — `/omd-phase-complete <target>` 跳过证据验证（保留 FSM 守卫）/ Skip evidence, keep FSM guards

---

## 典型工作流 / Typical Workflow

```
用户 → 启动 deepseek-omd
         │
    鸿钧 (Router)
    "分析你的需求..."
         │
    ┌────┴────┐
    ▼         ▼
  伏羲      通天
 (规划)    (直接执行)
    │
  Interview → Explore → Architect → Plan → Done
    │
  ┌─┴─ 一键交接 ───┐
  ▼                │
盘古 (执行)         │
  │                │
  LoadPlan → Decompose → Delegate → Verify → Done
                     │
              ┌──────┼──────┬──────┐
              ▼      ▼      ▼      ▼
           通天弟子  鲲鹏   女娲   神农 ...
           (实现)   (分析)  (验证)  (测试)
```

---

## 测试 / Testing

OMD 包含 **256 个测试** / 256 tests:

```bash
# 运行所有 OMD 测试 / Run all OMD tests
cargo test -p omd

# 运行特定模块 / Run specific module
cargo test -p omd shell_parser    # Shell 解析器 (101 tests)
cargo test -p omd evidence        # 证据验证
cargo test -p omd transition      # 转换守卫
cargo test -p omd golden          # 工具白名单黄金测试
cargo test -p omd tasks           # 任务图
cargo test -p omd runtime         # 运行时 + 崩溃恢复
```

---

## 开发里程碑 / Development Milestones

| 阶段 | 内容 | 状态 |
|------|------|:----:|
| Phase 1 (Plan 1-2) | 核心 FSM、4 智能体、7 工人、工具执行、写入范围、Shell 策略 | Done |
| Phase 2 (Plan 3) | 状态持久化、崩溃恢复、证据系统、安装脚本 | Done |
| Phase 2 (Plan 4) | 加固 — 锁文件、转换守卫、黄金测试、Codex 审查 | Done |
| Phase 2 (Plan 5) | Spec 100% 完成 — 解析器、GitDiff、用户门控、事件重放、交接组件 | Done |
| Phase 3 (规划中) | 扩展工人、Git 历史分析、自适应重试、跨会话知识共享 | Planned |

---

## 贡献 / Contributing

欢迎贡献！详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 致谢 / Acknowledgments

- 基于 [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) v0.8.36 (fork)
- 灵感来自 [OpenAgent](https://github.com/openagentinc/openagent) 和 OpenCode
- DeepSeek API: [DeepSeek AI](https://www.deepseek.com/)

## 许可证 / License

[MIT](LICENSE)
