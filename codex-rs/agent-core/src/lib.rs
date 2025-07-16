//! 

pub mod agent;
pub mod agents;
pub mod capability;
pub mod coordinator;
pub mod error;
pub mod manager;
pub mod scheduler;
pub mod task;

pub use agent::{Agent, AgentId, AgentType};
pub use agents::{AnalysisAgent, CodeAgent, TestAgent};
pub use capability::Capability;
pub use coordinator::{Coordinator, ExecutionStrategy};
pub use error::{AgentError, Result};
pub use manager::AgentManager;
pub use scheduler::{TaskScheduler, Priority};
pub use task::{Task, TaskId, TaskResult, TaskStatus};
