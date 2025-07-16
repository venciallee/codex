use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub type TaskId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub task_type: String,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub required_capabilities: Vec<String>,
    pub priority: Priority,
    pub dependencies: Vec<TaskId>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub status: TaskStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: Option<u64>,
    pub metadata: HashMap<String, String>,
}

impl Task {
    pub fn new(task_type: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type: task_type.into(),
            description: description.into(),
            parameters: HashMap::new(),
            required_capabilities: Vec::new(),
            priority: Priority::default(),
            dependencies: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capabilities.push(capability.into());
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_dependency(mut self, task_id: TaskId) -> Self {
        self.dependencies.push(task_id);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl TaskResult {
    pub fn success(task_id: TaskId, output: Option<serde_json::Value>) -> Self {
        Self {
            task_id,
            status: TaskStatus::Completed,
            output,
            error: None,
            execution_time_ms: None,
            metadata: HashMap::new(),
        }
    }

    pub fn failure(task_id: TaskId, error: impl Into<String>) -> Self {
        Self {
            task_id,
            status: TaskStatus::Failed(error.into()),
            output: None,
            error: Some(error.into()),
            execution_time_ms: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_execution_time(mut self, time_ms: u64) -> Self {
        self.execution_time_ms = Some(time_ms);
        self
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self.status, TaskStatus::Failed(_))
    }
}
