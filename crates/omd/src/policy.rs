use crate::types::*;

pub struct PhaseToolPolicy {
    allowed: &'static [&'static str],
    allow_all: bool,
}

/// All OMD control tools (always available in OMD modes)
const OMD_TOOLS: &[&str] = &["omd_phase_complete", "omd_checkpoint", "omd_state_read"];

/// Read-only tools (available in explore/verify phases)
#[allow(dead_code)]
const READ_TOOLS: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "git_status", "git_diff", "git_log", "git_show", "git_blame",
    "diagnostics",
];

impl PhaseToolPolicy {
    pub fn for_phase(phase: &OmdPhase) -> Self {
        match phase {
            OmdPhase::Tongtian(p) => match p {
                TongtianPhase::Explore => Self { allowed: &TONGTIAN_EXPLORE, allow_all: false },
                TongtianPhase::Execute => Self { allowed: &[], allow_all: true },
                TongtianPhase::Verify => Self { allowed: &TONGTIAN_VERIFY, allow_all: false },
                TongtianPhase::Done => Self { allowed: OMD_TOOLS, allow_all: false },
            },
            OmdPhase::Fuxi(p) => match p {
                FuxiPhase::Interview => Self { allowed: &FUXI_INTERVIEW, allow_all: false },
                FuxiPhase::Explore => Self { allowed: &FUXI_EXPLORE, allow_all: false },
                FuxiPhase::Architect => Self { allowed: &FUXI_ARCHITECT, allow_all: false },
                FuxiPhase::Plan => Self { allowed: &FUXI_PLAN, allow_all: false },
                FuxiPhase::Done => Self { allowed: OMD_TOOLS, allow_all: false },
            },
            OmdPhase::Pangu(p) => match p {
                PanguPhase::LoadPlan => Self { allowed: &PANGU_LOAD_PLAN, allow_all: false },
                PanguPhase::Decompose => Self { allowed: &PANGU_DECOMPOSE, allow_all: false },
                PanguPhase::Delegate => Self { allowed: &PANGU_DELEGATE, allow_all: false },
                PanguPhase::Verify => Self { allowed: &PANGU_VERIFY, allow_all: false },
                PanguPhase::Done => Self { allowed: OMD_TOOLS, allow_all: false },
            },
            OmdPhase::Hongjun(p) => match p {
                HongjunPhase::Intake => Self { allowed: &HONGJUN_INTAKE, allow_all: false },
                HongjunPhase::Route => Self { allowed: &HONGJUN_ROUTE, allow_all: false },
                HongjunPhase::Done => Self { allowed: OMD_TOOLS, allow_all: false },
            },
        }
    }

    pub fn is_allowed(&self, tool_name: &str) -> bool {
        self.allow_all || self.allowed.contains(&tool_name)
    }

    pub fn is_allow_all(&self) -> bool { self.allow_all }

    pub fn allowed_list(&self) -> &[&str] { self.allowed }
}

// ── Tongtian phase allowlists ─────────────────────────────────

static TONGTIAN_EXPLORE: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "git_status", "git_diff", "git_log", "git_show", "git_blame",
    "diagnostics",
    "omd_phase_complete", "omd_checkpoint", "omd_state_read",
];

static TONGTIAN_VERIFY: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "git_status", "git_diff", "git_log", "git_show",
    "exec_shell", "exec_shell_wait",
    "omd_phase_complete", "omd_checkpoint", "omd_state_read",
];

// ── Fuxi phase allowlists ─────────────────────────────────

static FUXI_INTERVIEW: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "omd_phase_complete", "omd_state_read",
];
static FUXI_EXPLORE: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "git_log", "git_diff", "git_show", "git_blame",
    "omd_phase_complete", "omd_state_read",
];
static FUXI_ARCHITECT: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "git_log", "git_diff",
    "omd_phase_complete", "omd_state_read",
];
static FUXI_PLAN: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "write_file",  // For writing .omd/plans/ (path validation is Plan 3)
    "omd_phase_complete", "omd_state_read",
];

// ── Pangu phase allowlists ────────────────────────────────

static PANGU_LOAD_PLAN: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "omd_phase_complete", "omd_state_read",
];
static PANGU_DECOMPOSE: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "omd_phase_complete", "omd_state_read",
];
static PANGU_DELEGATE: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "omd_delegate", "agent_eval", "agent_close",
    "omd_phase_complete", "omd_checkpoint", "omd_state_read",
];
static PANGU_VERIFY: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "exec_shell", "exec_shell_wait",
    "omd_delegate", "agent_eval", "agent_close",
    "omd_phase_complete", "omd_checkpoint", "omd_state_read",
];

// ── Hongjun phase allowlists ──────────────────────────────

static HONGJUN_INTAKE: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "omd_phase_complete", "omd_state_read",
];
static HONGJUN_ROUTE: &[&str] = &[
    "read_file",
    "omd_phase_complete", "omd_state_read",
];
