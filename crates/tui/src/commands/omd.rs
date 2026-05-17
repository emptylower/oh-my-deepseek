//! OMD slash commands: /omd-execute and /omd-phase-complete

use crate::tui::app::{App, AppMode};
use super::CommandResult;

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
/// User escape hatch for stalled phases.
pub fn omd_phase_complete(app: &mut App, arg: Option<&str>) -> CommandResult {
    if !matches!(app.mode, AppMode::OmdTongtian | AppMode::OmdFuxi | AppMode::OmdPangu | AppMode::OmdHongjun) {
        return CommandResult::error("Not in an OMD mode. Switch to an OMD mode first.");
    }

    match arg {
        Some(target) => {
            // Inject a message for the model to call omd_phase_complete
            CommandResult::message(format!(
                "Phase transition requested: target='{}'. The model will call omd_phase_complete tool to execute the transition.",
                target.trim()
            ))
        }
        None => {
            CommandResult::message(
                "Usage: /omd-phase-complete <target_phase>. This is an escape hatch for stalled phases."
                    .to_string()
            )
        }
    }
}
