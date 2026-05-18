use serde::Serialize;

/// Configuration for a worker agent that can be spawned by Pangu via omd_delegate.
///
/// Note: Only Serialize is derived (for omd_state_read output).
/// Static configs don't need Deserialize — they're defined in code, not loaded from disk.
#[derive(Debug, Clone, Serialize)]
pub struct OmdWorkerConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub role_description: &'static str,
    pub system_prompt_prefix: &'static str,
    pub allowed_tools: &'static [&'static str],
    pub can_write_code: bool,
    pub can_delegate: bool,
    /// Reasoning effort level for this worker: "off", "high", or "max"
    pub reasoning_effort: &'static str,
}

/// Registry of all available worker agents.
pub struct WorkerRegistry {
    workers: &'static [OmdWorkerConfig],
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self { workers: ALL_WORKERS }
    }

    pub fn get(&self, id: &str) -> Option<&OmdWorkerConfig> {
        self.workers.iter().find(|w| w.id == id)
    }

    pub fn all(&self) -> &[OmdWorkerConfig] {
        self.workers
    }
}

/// All worker definitions.
///
/// IMPORTANT: No worker has `omd_checkpoint`, `omd_delegate`, or `agent_open`.
/// Sub-agent runtimes don't register OMD tools, and workers cannot re-delegate.
static ALL_WORKERS: &[OmdWorkerConfig] = &[
    OmdWorkerConfig {
        id: "tongtian-junior",
        display_name: "通天 Junior",
        role_description: "Code executor — implements specific tasks",
        system_prompt_prefix: "You are Tongtian-Junior, a focused code executor. Your job is to implement the specific task assigned to you cleanly and completely. Write code, run tests, verify your work. Do NOT explore beyond the task scope or refactor unrelated code.",
        allowed_tools: &[
            "read_file", "grep_files", "file_search", "list_dir",
            "edit_file", "write_file", "apply_patch",
            "exec_shell", "exec_shell_wait",
            "git_status", "git_diff",
        ],
        can_write_code: true,
        can_delegate: false,
        reasoning_effort: "high",
    },
    OmdWorkerConfig {
        id: "kunpeng",
        display_name: "鲲鹏 Kunpeng",
        role_description: "Explorer — deep codebase analysis",
        system_prompt_prefix: "You are Kunpeng, a codebase explorer. Your job is to thoroughly explore and analyze the codebase to answer questions or find patterns. Report your findings clearly and concisely. Do NOT modify any files.",
        allowed_tools: &[
            "read_file", "grep_files", "file_search", "list_dir",
            "git_log", "git_diff", "git_show", "git_blame",
        ],
        can_write_code: false,
        can_delegate: false,
        reasoning_effort: "off",
    },
    OmdWorkerConfig {
        id: "nuwa",
        display_name: "女娲 Nuwa",
        role_description: "Verifier — runs tests and validates changes",
        system_prompt_prefix: "You are Nuwa, a verifier. Your job is to verify that implementation work is correct by running tests, checking build status, and validating behavior. Report pass/fail with evidence. Do NOT fix issues — only verify and report.",
        allowed_tools: &[
            "read_file", "grep_files", "file_search", "list_dir",
            "exec_shell", "exec_shell_wait",
            "git_status", "git_diff",
        ],
        can_write_code: false,
        can_delegate: false,
        reasoning_effort: "high",
    },
    OmdWorkerConfig {
        id: "shennong",
        display_name: "神农 Shennong",
        role_description: "Test engineer — writes and runs tests",
        system_prompt_prefix: "You are Shennong, a test engineer. Your job is to write tests for the specified functionality, run them, and report results. Focus on test quality: meaningful assertions, edge cases, clear test names.",
        allowed_tools: &[
            "read_file", "grep_files", "file_search", "list_dir",
            "write_file", "edit_file",
            "exec_shell", "exec_shell_wait",
            "git_status", "git_diff",
        ],
        can_write_code: true,
        can_delegate: false,
        reasoning_effort: "high",
    },
    OmdWorkerConfig {
        id: "yangmei",
        display_name: "杨眉 Yangmei",
        role_description: "Critic/reviewer — reviews code quality",
        system_prompt_prefix: "You are Yangmei, a code reviewer. Your job is to review code changes for correctness, style, and potential issues. Provide specific feedback with file:line references. Do NOT modify any files.",
        allowed_tools: &[
            "read_file", "grep_files", "file_search", "list_dir",
            "git_diff", "git_log", "git_show",
        ],
        can_write_code: false,
        can_delegate: false,
        reasoning_effort: "off",
    },
    OmdWorkerConfig {
        id: "cangjie",
        display_name: "仓颉 Cangjie",
        role_description: "Documentation writer — creates and updates docs",
        system_prompt_prefix: "You are Cangjie, a documentation writer. Your job is to create or update documentation files (.md, comments, README). Write clearly and concisely. Only modify documentation files.",
        allowed_tools: &[
            "read_file", "grep_files", "file_search", "list_dir",
            "write_file", "edit_file",
            "git_status", "git_diff",
        ],
        can_write_code: true,
        can_delegate: false,
        reasoning_effort: "off",
    },
    OmdWorkerConfig {
        id: "zhurong",
        display_name: "祝融 Zhurong",
        role_description: "Debugger — diagnoses and traces issues",
        system_prompt_prefix: "You are Zhurong, a debugger. Your job is to diagnose issues by reading code, running targeted commands, and tracing execution flow. Report root cause analysis. Do NOT fix issues — only diagnose and report.",
        allowed_tools: &[
            "read_file", "grep_files", "file_search", "list_dir",
            "exec_shell", "exec_shell_wait",
            "git_log", "git_diff", "git_show", "git_blame",
        ],
        can_write_code: false,
        can_delegate: false,
        reasoning_effort: "high",
    },
];
