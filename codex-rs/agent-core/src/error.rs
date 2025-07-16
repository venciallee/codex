use thiserror::Error;

pub type Result<T> = std::result::Result<T, AgentError>;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),

    #[error("Invalid task configuration: {0}")]
    InvalidTaskConfig(String),

    #[error("Agent capability mismatch: required {required}, available {available:?}")]
    CapabilityMismatch {
        required: String,
        available: Vec<String>,
    },

    #[error("Coordination error: {0}")]
    CoordinationError(String),

    #[error("Scheduling error: {0}")]
    SchedulingError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}
