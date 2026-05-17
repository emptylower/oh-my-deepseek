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
            // Stubs for Plan 2
            OmdPhase::Fuxi(_) => Self { allowed: &FUXI_DEFAULT, allow_all: false },
            OmdPhase::Pangu(_) => Self { allowed: &PANGU_DEFAULT, allow_all: false },
            OmdPhase::Hongjun(_) => Self { allowed: &HONGJUN_DEFAULT, allow_all: false },
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

// ── Stubs for Plan 2 ─────────────────────────────────────────

static FUXI_DEFAULT: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "git_log", "git_diff", "git_status",
    "omd_phase_complete", "omd_checkpoint", "omd_state_read",
];

static PANGU_DEFAULT: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "omd_phase_complete", "omd_checkpoint", "omd_state_read", "omd_delegate",
];

static HONGJUN_DEFAULT: &[&str] = &[
    "read_file", "grep_files", "file_search", "list_dir",
    "omd_phase_complete", "omd_state_read",
];
