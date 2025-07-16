use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::capability::CapabilitySet;
use crate::error::Result;
use crate::task::{Task, TaskResult};

pub type AgentId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    CodeAgent,
    AnalysisAgent,
    TestAgent,
    DocAgent,
    SecurityAgent,
    CoordinatorAgent,
    Custom(String),
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentType::CodeAgent => write!(f, "code"),
            AgentType::AnalysisAgent => write!(f, "analysis"),
            AgentType::TestAgent => write!(f, "test"),
            AgentType::DocAgent => write!(f, "doc"),
            AgentType::SecurityAgent => write!(f, "security"),
            AgentType::CoordinatorAgent => write!(f, "coordinator"),
            AgentType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> AgentId;

    fn agent_type(&self) -> AgentType;

    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn capabilities(&self) -> &CapabilitySet;

    fn can_handle(&self, task: &Task) -> bool {
        self.capabilities()
            .matches_requirements(&task.required_capabilities)
    }

    async fn execute(&self, task: Task) -> Result<TaskResult>;

    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    async fn health_check(&self) -> Result<AgentHealth> {
        Ok(AgentHealth::Healthy)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentHealth {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

impl AgentHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, AgentHealth::Healthy)
    }

    pub fn is_degraded(&self) -> bool {
        matches!(self, AgentHealth::Degraded(_))
    }

    pub fn is_unhealthy(&self) -> bool {
        matches!(self, AgentHealth::Unhealthy(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub agent_type: AgentType,
    pub name: String,
    pub description: String,
    pub capabilities: CapabilitySet,
    pub health: AgentHealth,
    pub version: String,
    pub created_at: std::time::SystemTime,
}

impl AgentInfo {
    pub fn new(agent: &dyn Agent) -> Self {
        Self {
            id: agent.id(),
            agent_type: agent.agent_type(),
            name: agent.name().to_string(),
            description: agent.description().to_string(),
            capabilities: agent.capabilities().clone(),
            health: AgentHealth::Healthy,
            version: "1.0.0".to_string(),
            created_at: std::time::SystemTime::now(),
        }
    }
}
