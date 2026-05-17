use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Blocked,
    Active,
    Done,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub owner: Option<String>,
    pub depends_on: Vec<String>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub failure_reason: Option<String>,
    pub changed_files: Vec<String>,
    /// Verified evidence collected from workers (test output, build logs, etc.)
    pub evidence: Vec<Value>,
    /// Allowed file paths/globs for this task's write operations
    pub write_scope: Vec<String>,
    /// Task category: implementation, test, explore, or debug
    pub category: Option<String>,
}

impl Task {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            status: TaskStatus::Pending,
            owner: None,
            depends_on: Vec::new(),
            attempts: 0,
            max_attempts: 3,
            failure_reason: None,
            changed_files: Vec::new(),
            evidence: Vec::new(),
            write_scope: Vec::new(),
            category: None,
        }
    }
}

pub fn is_valid_transition(from: &TaskStatus, to: &TaskStatus) -> bool {
    matches!(
        (from, to),
        (TaskStatus::Pending, TaskStatus::Active)
            | (TaskStatus::Pending, TaskStatus::Skipped)
            | (TaskStatus::Active, TaskStatus::Done)
            | (TaskStatus::Active, TaskStatus::Failed)
            | (TaskStatus::Failed, TaskStatus::Active) // retry
            | (TaskStatus::Blocked, TaskStatus::Pending) // auto-unblock
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskGraph {
    pub tasks: Vec<Task>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Add a task without validation. Call validate() after adding all tasks.
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Validate the full graph: unique IDs, deps exist, no cycles.
    pub fn validate(&self) -> Result<(), String> {
        let ids: std::collections::HashSet<&str> =
            self.tasks.iter().map(|t| t.id.as_str()).collect();

        // Check unique IDs
        if ids.len() != self.tasks.len() {
            let mut seen = std::collections::HashSet::new();
            for task in &self.tasks {
                if !seen.insert(&task.id) {
                    return Err(format!("Duplicate task ID: {}", task.id));
                }
            }
        }

        // Check deps exist
        for task in &self.tasks {
            for dep in &task.depends_on {
                if !ids.contains(dep.as_str()) {
                    return Err(format!(
                        "Task '{}' depends on non-existent '{}'",
                        task.id, dep
                    ));
                }
            }
        }

        // Cycle detection via Kahn's algorithm (topological sort)
        // depends_on means "this task depends on dep", so dep must complete first.
        // Edge: dep → task. in_degree[task] = number of deps.
        let mut in_degree: std::collections::HashMap<&str, usize> =
            ids.iter().map(|id| (*id, 0usize)).collect();
        for task in &self.tasks {
            *in_degree.get_mut(task.id.as_str()).unwrap() = task.depends_on.len();
        }
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut visited = 0usize;
        while let Some(node) = queue.pop() {
            visited += 1;
            // Find tasks that depend on this node and decrement their in-degree
            for task in &self.tasks {
                if task.depends_on.iter().any(|d| d == node) {
                    let deg = in_degree.get_mut(task.id.as_str()).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(&task.id);
                    }
                }
            }
        }
        if visited != self.tasks.len() {
            return Err("Task graph contains a cycle".to_string());
        }

        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn set_status(&mut self, id: &str, status: TaskStatus) -> Result<(), String> {
        // Enforce max 1 active task at a time
        if status == TaskStatus::Active {
            let active_count = self.tasks.iter()
                .filter(|t| t.status == TaskStatus::Active && t.id != id)
                .count();
            if active_count >= 1 {
                return Err(format!("Cannot activate '{}': max 1 active task at a time", id));
            }
        }

        let task = self.get_mut(id)
            .ok_or_else(|| format!("Task '{}' not found", id))?;

        // Reject retry if attempts exhausted
        if task.status == TaskStatus::Failed && status == TaskStatus::Active {
            if task.attempts >= task.max_attempts {
                return Err(format!("Task '{}' exhausted {} retries", id, task.max_attempts));
            }
        }

        // Validate transition
        if !is_valid_transition(&task.status, &status) {
            return Err(format!(
                "Invalid transition for '{}': {:?} → {:?}",
                id, task.status, status
            ));
        }

        // Track retry attempts
        if task.status == TaskStatus::Active && status == TaskStatus::Failed {
            task.attempts += 1;
        }

        task.status = status;
        Ok(())
    }

    /// Returns the ID of the next task that can run (Pending + all deps Done).
    pub fn next_runnable(&self) -> Option<String> {
        self.tasks
            .iter()
            .find(|t| {
                t.status == TaskStatus::Pending
                    && t.depends_on.iter().all(|dep| {
                        self.get(dep)
                            .map_or(false, |d| d.status == TaskStatus::Done)
                    })
            })
            .map(|t| t.id.clone())
    }

    pub fn all_done(&self) -> bool {
        self.tasks
            .iter()
            .all(|t| matches!(t.status, TaskStatus::Done | TaskStatus::Skipped))
    }

    pub fn progress(&self) -> (usize, usize) {
        let done = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Done)
            .count();
        (done, self.tasks.len())
    }
}
