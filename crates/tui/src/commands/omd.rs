//! OMD slash commands: /omd-debug, /omd-execute, and /omd-phase-complete

use crate::tui::app::{App, AppAction, AppMode};
use super::CommandResult;

/// Handle /omd-debug
/// Reads `.omd/debug.jsonl` and the current session state, then displays
/// a formatted summary of recent OMD events for debugging.
pub fn omd_debug(app: &mut App) -> CommandResult {
    let workspace = &app.workspace;
    let debug_logger = omd::OmdDebugLogger::new(workspace);

    // Try to read current session state for context
    let store = omd::state::OmdStateStore::new(workspace);
    let (agent, phase, session_id, started_at, available_tools_owned) = match store.read_state() {
        Ok(Some(state)) => {
            // Resolve available tools from the phase policy
            let phase_obj = omd::types::OmdPhase::from_agent_and_name(&state.agent, &state.phase);
            let tools: Vec<String> = if let Some(ref p) = phase_obj {
                omd::PhaseToolPolicy::for_phase(p)
                    .allowed_list()
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                Vec::new()
            };
            (state.agent, state.phase, state.session_id, state.started_at, tools)
        }
        _ => {
            // No active session — still show the debug log if it exists
            let entries = debug_logger.recent_entries(20);
            if entries.is_empty() {
                return CommandResult::message(
                    "=== OMD Debug Status ===\nNo active OMD session and no debug events found.\n\
                     Start an OMD mode to begin logging."
                );
            }
            ("unknown".to_string(), "unknown".to_string(), "none".to_string(),
             "unknown".to_string(), Vec::new())
        }
    };

    let available_tools: Vec<&str> = available_tools_owned.iter().map(|s| s.as_str()).collect();
    let summary = debug_logger.format_summary(
        &agent,
        &phase,
        &session_id,
        &started_at,
        &available_tools,
    );

    CommandResult::message(summary)
}

/// Handle /omd-execute [plan-name]
/// Switches to OmdPangu mode and shows the plan path.
pub fn omd_execute(app: &mut App, arg: Option<&str>) -> CommandResult {
    let plan_path = if let Some(path) = arg {
        path.to_string()
    } else {
        // Find most recent plan in .omd/plans/
        let plans_dir = app.workspace.join(".omd/plans");
        if plans_dir.exists() {
            let mut entries: Vec<_> = std::fs::read_dir(&plans_dir)
                .ok()
                .map(|rd| rd.filter_map(|e| e.ok()).collect())
                .unwrap_or_default();
            entries.sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));
            match entries.first() {
                Some(e) => e.path().to_string_lossy().to_string(),
                None => return CommandResult::error("No plans found in .omd/plans/. Create one with Fuxi first."),
            }
        } else {
            return CommandResult::error("No .omd/plans/ directory. Create a plan with Fuxi first, or specify: /omd-execute <path>");
        }
    };

    let _ = app.set_mode(AppMode::OmdPangu);
    CommandResult::message(format!("Switched to Pangu mode. Plan: {}", plan_path))
}

/// Handle /omd-phase-complete [target_phase]
/// User escape hatch for stalled phases. Bypasses evidence verification
/// but enforces FSM validity and structural transition guards.
pub fn omd_phase_complete(app: &mut App, arg: Option<&str>) -> CommandResult {
    if !matches!(app.mode, AppMode::OmdTongtian | AppMode::OmdFuxi | AppMode::OmdPangu | AppMode::OmdHongjun) {
        return CommandResult::error("Not in an OMD mode. Switch to an OMD mode first.");
    }

    match arg {
        Some(target) => CommandResult::action(AppAction::OmdPhaseComplete {
            target_phase: target.trim().to_string(),
        }),
        None => CommandResult::error(
            "Usage: /omd-phase-complete <target_phase>. This is an escape hatch for stalled phases."
        ),
    }
}
