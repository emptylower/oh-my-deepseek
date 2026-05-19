//! omd_delegate — Pangu's delegation tool.
//!
//! Wraps TUI's internal `AgentSpawnTool` with OMD policy injection.
//! Returns the session handle immediately — Pangu calls `agent_eval`
//! separately to poll/wait for worker completion.

use async_trait::async_trait;
use omd::{SharedOmdRuntime, WorkerRegistry};
use serde_json::{json, Value};

use super::spec::{ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec};
use super::subagent::{AgentSpawnTool, SharedSubAgentManager, SubAgentRuntime};

pub struct OmdDelegateTool {
    runtime: SharedOmdRuntime,
    manager: SharedSubAgentManager,
    subagent_runtime: SubAgentRuntime,
}

impl OmdDelegateTool {
    pub fn new(
        runtime: SharedOmdRuntime,
        manager: SharedSubAgentManager,
        subagent_runtime: SubAgentRuntime,
    ) -> Self {
        Self { runtime, manager, subagent_runtime }
    }
}

#[async_trait]
impl ToolSpec for OmdDelegateTool {
    fn name(&self) -> &str { "omd_delegate" }

    fn description(&self) -> &str {
        "Delegate a bounded task to a worker agent. Returns the session handle immediately — \
         use `agent_eval` to poll/wait for the worker's result. Only available to Pangu in \
         Delegate/Verify phases. Workers have restricted tool access and cannot re-delegate."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "agent": {
                    "type": "string",
                    "description": "Worker agent ID: tongtian-junior, kunpeng, nuwa, shennong, yangmei, cangjie, zhurong"
                },
                "task": {
                    "type": "string",
                    "description": "Specific bounded task description for the worker"
                },
                "context": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Relevant file paths or context to pass to the worker"
                },
                "category": {
                    "type": "string",
                    "enum": ["implementation", "test", "explore", "debug"],
                    "description": "Task category for tracking"
                },
                "task_id": {
                    "type": "string",
                    "description": "Task graph ID for tracking (matches TaskGraph task IDs)"
                },
                "write_scope": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Allowed file paths/globs the worker may write to"
                }
            },
            "required": ["agent", "task"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ExecutesCode]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let agent_id = input.get("agent").and_then(|v| v.as_str()).unwrap_or("");
        let task = input.get("task").and_then(|v| v.as_str()).unwrap_or("");
        let ctx_paths: Vec<String> = input.get("context")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let category = input.get("category").and_then(|v| v.as_str()).map(String::from);
        let task_id = input.get("task_id").and_then(|v| v.as_str()).map(String::from);
        let write_scope: Vec<String> = input.get("write_scope")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Validate worker exists
        let registry = WorkerRegistry::new();
        let worker = registry.get(agent_id).ok_or_else(|| {
            ToolError::invalid_input(format!(
                "Unknown worker '{}'. Available: {:?}",
                agent_id,
                registry.all().iter().map(|w| w.id).collect::<Vec<_>>()
            ))
        })?;

        // Worker delegation restrictions per agent/phase
        {
            let state = self.runtime.read().await;
            let agent = format!("{:?}", state.fsm.agent());
            let phase_name = state.fsm.current_phase_name();

            // Read-only workers only — for agents gathering information (not executing)
            const READ_ONLY_WORKERS: &[&str] = &["kunpeng", "yangmei", "nuwa", "tingfeng"];

            match (agent.as_str(), phase_name) {
                // Pangu Verify: only Nuwa
                (_, "Verify") => {
                    if agent_id != "nuwa" {
                        return Err(ToolError::permission_denied(format!(
                            "In Verify phase, only 'nuwa' can be delegated to (got '{}')", agent_id
                        )));
                    }
                }
                // Fuxi: only read-only workers (information gathering, not implementation)
                ("Fuxi", _) => {
                    if !READ_ONLY_WORKERS.contains(&agent_id) {
                        return Err(ToolError::permission_denied(format!(
                            "Fuxi can only delegate to read-only workers ({:?}). Got '{}'. \
                             Fuxi gathers information — implementation is Pangu's job.",
                            READ_ONLY_WORKERS, agent_id
                        )));
                    }
                }
                // Tongtian Explore: only read-only workers
                ("Tongtian", "Explore") => {
                    if !READ_ONLY_WORKERS.contains(&agent_id) {
                        return Err(ToolError::permission_denied(format!(
                            "In Explore phase, only read-only workers ({:?}) can be delegated to. Got '{}'.",
                            READ_ONLY_WORKERS, agent_id
                        )));
                    }
                }
                // Pangu Delegate: all workers allowed
                _ => {}
            }
        }

        // Build prompt with role context
        let context_section = if ctx_paths.is_empty() {
            String::new()
        } else {
            format!("\n\nRelevant files:\n{}", ctx_paths.iter().map(|p| format!("- {p}")).collect::<Vec<_>>().join("\n"))
        };

        let scope_section = if write_scope.is_empty() {
            String::new()
        } else {
            format!("\n\nAllowed write scope:\n{}", write_scope.iter().map(|p| format!("- {p}")).collect::<Vec<_>>().join("\n"))
        };

        let full_prompt = format!(
            "{}\n\n## Task\n\n{}{}{}",
            worker.system_prompt_prefix, task, context_section, scope_section
        );

        // Spawn via native agent_spawn with custom type + explicit allowed_tools.
        let session_name = format!("omd-{}-{}", agent_id, chrono::Utc::now().timestamp());
        let spawn_input = json!({
            "prompt": full_prompt,
            "type": "custom",
            "allowed_tools": worker.allowed_tools,
            "name": session_name,
            "fork_context": false,
        });

        // Build a runtime with OMD scope for this worker
        let mut worker_runtime = self.subagent_runtime.clone();
        if !write_scope.is_empty() {
            worker_runtime.omd_write_scope = Some(write_scope.clone());
            // Workers with write_scope get read-only shell (prevents shell-based file writes)
            worker_runtime.omd_shell_read_only = true;
        } else if worker.can_write_code {
            // Writable worker without explicit scope — log for audit.
            // Write-scope is opt-in hardening; workers are still bounded by allowed_tools.
            tracing::warn!(
                "omd_delegate: writable worker '{}' spawned without write_scope. \
                 File-level enforcement disabled for this delegation.",
                agent_id
            );
        }
        // Set reasoning effort from worker config
        worker_runtime.reasoning_effort = Some(worker.reasoning_effort.to_string());
        // Fix: allow immediate child spawn (max_spawn_depth must be > spawn_depth)
        // Workers can't recurse further because agent_spawn is not in their allowed_tools
        worker_runtime.max_spawn_depth = worker_runtime.spawn_depth + 1;

        let spawn_tool = AgentSpawnTool::new(self.manager.clone(), worker_runtime);
        let result = spawn_tool.execute(spawn_input, context).await?;

        // Log delegation event
        {
            let state = self.runtime.read().await;
            let _ = state.store.append_event(
                &state.session_state.session_id,
                &json!({
                    "ts": chrono::Utc::now().to_rfc3339(),
                    "event": "delegate",
                    "agent": agent_id,
                    "task": task,
                    "task_id": task_id,
                    "category": category,
                    "write_scope": write_scope,
                    "context": ctx_paths,
                    "session_name": session_name,
                }),
            );
        }

        // Return enriched result with session name for agent_eval
        Ok(ToolResult {
            success: true,
            content: format!(
                "Spawned worker {} (session: {}). Use `agent_eval` with session name '{}' to wait for results.",
                worker.display_name, session_name, session_name
            ),
            metadata: result.metadata,
        })
    }
}
