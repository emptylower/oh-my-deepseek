//! Per-turn tool registry setup.
//!
//! This keeps mode/feature-specific registry construction out of the send path.

use std::path::Path;

use super::*;
use crate::sandbox::SandboxPolicy;

/// Pick the sandbox policy that gates shell commands for a given UI mode.
///
/// - **Plan** (#1077): `ReadOnly` — no writes, no network. The previous
///   `WorkspaceWrite` policy let `python -c "open('f','w').write('x')"` mutate
///   files inside the workspace because it whitelisted the workspace as
///   writable. Plan mode is investigation only; if the user wants to change
///   files they should switch to Agent.
/// - **Agent**: `WorkspaceWrite` with workspace as writable root and network
///   on. Approval flow gates risky individual commands; the sandbox handles
///   the rest. Network is allowed because cargo / npm / curl-style commands
///   are normal during agent work and DNS-deny breaks them silently.
/// - **YOLO**: `DangerFullAccess` — explicit no-guardrails contract.
pub(crate) fn sandbox_policy_for_mode(mode: AppMode, workspace: &Path) -> SandboxPolicy {
    match mode {
        AppMode::Plan | AppMode::OmdFuxi | AppMode::OmdHongjun => SandboxPolicy::ReadOnly,
        AppMode::Agent => SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![workspace.to_path_buf()],
            network_access: true,
            exclude_tmpdir: false,
            exclude_slash_tmp: false,
        },
        AppMode::Yolo | AppMode::OmdTongtian | AppMode::OmdPangu => SandboxPolicy::DangerFullAccess,
    }
}

impl Engine {
    pub(super) fn build_turn_tool_registry_builder(
        &self,
        mode: AppMode,
        todo_list: SharedTodoList,
        plan_state: SharedPlanState,
    ) -> ToolRegistryBuilder {
        // OMD restricted phases: return exact registry immediately.
        // The model sees ONLY phase-allowed tools — no common tools appended.
        if matches!(mode, AppMode::OmdTongtian | AppMode::OmdFuxi | AppMode::OmdPangu | AppMode::OmdHongjun) {
            if let Some(ref omd_rt) = self.omd_runtime {
                let phase = {
                    let state = tokio::task::block_in_place(|| omd_rt.blocking_read());
                    *state.fsm.phase()
                };
                let policy = omd::PhaseToolPolicy::for_phase(&phase);

                if !policy.is_allow_all() {
                    // Restricted phase (Explore/Verify/Done): exact registry, early return.
                    let mut b = ToolRegistryBuilder::new()
                        .with_tool(Arc::new(crate::tools::omd::OmdPhaseCompleteTool::new(omd_rt.clone())))
                        .with_tool(Arc::new(crate::tools::omd::OmdCheckpointTool::new(omd_rt.clone())))
                        .with_tool(Arc::new(crate::tools::omd::OmdStateReadTool::new(omd_rt.clone())))
                        .with_read_only_file_tools()
                        .with_search_tools()
                        .with_git_tools();
                    if policy.is_allowed("exec_shell") {
                        b = b.with_shell_tools();
                    }
                    if policy.is_allowed("write_file") {
                        // Fuxi Plan phase needs write_file to output .omd/plans/
                        b = b.with_tool(Arc::new(crate::tools::file::WriteFileTool));
                    }
                    if policy.is_allowed("omd_delegate") {
                        // Pangu's Delegate/Verify phases get delegation tools
                        if let Some(client) = self.deepseek_client.clone() {
                            let tool_ctx = ToolContext::new(self.session.workspace.clone());
                            let sa_runtime = SubAgentRuntime::new(
                                client,
                                self.session.model.clone(),
                                tool_ctx,
                                self.session.allow_shell,
                                Some(self.tx_event.clone()),
                                Arc::clone(&self.subagent_manager),
                            )
                            .with_max_spawn_depth(0);

                            // OMD delegation tool (with worker restrictions)
                            b = b
                                .with_tool(Arc::new(
                                    crate::tools::omd_delegate::OmdDelegateTool::new(
                                        omd_rt.clone(),
                                        self.subagent_manager.clone(),
                                        sa_runtime.clone(),
                                    ),
                                ))
                                .with_tool(Arc::new(
                                    crate::tools::subagent::AgentEvalTool::new(
                                        self.subagent_manager.clone(),
                                    ),
                                ))
                                .with_tool(Arc::new(
                                    crate::tools::subagent::AgentCloseTool::new(
                                        self.subagent_manager.clone(),
                                    ),
                                ));
                            // Native agent_open — model prefers this name from training.
                            // Works alongside omd_delegate; doesn't have worker restrictions.
                            b = b.with_tool(Arc::new(
                                crate::tools::subagent::AgentSpawnTool::new(
                                    self.subagent_manager.clone(),
                                    sa_runtime,
                                ),
                            ));
                        }
                    }
                    return b;
                }
                // Execute phase (is_allow_all): fall through to get full tooling + common tools.
            }
        }

        // OMD-Execute, Agent, Yolo, Plan: standard registry construction.
        let mut builder = if matches!(mode, AppMode::OmdTongtian | AppMode::OmdFuxi | AppMode::OmdPangu | AppMode::OmdHongjun) {
            // Must be Execute/allow_all phase (restricted returned above).
            let omd_rt = self.omd_runtime.as_ref().unwrap();
            ToolRegistryBuilder::new()
                .with_tool(Arc::new(crate::tools::omd::OmdPhaseCompleteTool::new(omd_rt.clone())))
                .with_tool(Arc::new(crate::tools::omd::OmdCheckpointTool::new(omd_rt.clone())))
                .with_tool(Arc::new(crate::tools::omd::OmdStateReadTool::new(omd_rt.clone())))
                .with_agent_tools(self.session.allow_shell)
        } else if mode == AppMode::Plan {
            ToolRegistryBuilder::new()
                .with_read_only_file_tools()
                .with_search_tools()
                .with_git_tools()
                .with_git_history_tools()
                .with_diagnostics_tool()
                .with_skill_tools()
                .with_validation_tools()
                .with_handle_tools()
                .with_runtime_read_only_task_tools()
                .with_todo_tool(todo_list)
                .with_plan_tool(plan_state)
        } else {
            ToolRegistryBuilder::new()
                .with_agent_tools(self.session.allow_shell)
                .with_todo_tool(todo_list)
                .with_plan_tool(plan_state)
        };

        builder = builder
            .with_review_tool(self.deepseek_client.clone(), self.session.model.clone())
            .with_user_input_tool()
            .with_parallel_tool()
            .with_recall_archive_tool();

        if mode != AppMode::Plan {
            builder = builder
                .with_rlm_tool(self.deepseek_client.clone(), self.session.model.clone())
                .with_fim_tool(self.deepseek_client.clone(), self.session.model.clone());
        }

        if self.config.features.enabled(Feature::ApplyPatch) && mode != AppMode::Plan {
            builder = builder.with_patch_tools();
        }
        if self.config.features.enabled(Feature::WebSearch) {
            builder = builder.with_web_tools();
        }
        // Plan mode is strictly read-only: do not expose shell execution at
        // all, even if the session would otherwise allow it.
        if mode != AppMode::Plan
            && self.config.features.enabled(Feature::ShellTool)
            && self.session.allow_shell
        {
            builder = builder.with_shell_tools();
        }

        // Register the `remember` tool only when the user has opted in to
        // user-memory (#489). Without that opt-in the tool would always
        // fail; surfacing it would just waste catalog slots.
        if self.config.memory_enabled {
            builder = builder.with_remember_tool();
        }

        // Register image_analyze tool when vision_model is configured and feature enabled.
        if self.config.features.enabled(Feature::VisionModel)
            && let Some(ref vision_config) = self.config.vision_config
        {
            builder = builder.with_vision_tools(vision_config.clone());
        }

        // Register the `notify` tool unconditionally (#1322). It has no
        // side effects beyond a single terminal escape write and respects
        // the user's `[notifications].method` config (including `off`),
        // so there's no failure mode worth gating on.
        builder = builder.with_notify_tool();

        builder
    }
}
