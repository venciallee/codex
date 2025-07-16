use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::agent::{Agent, AgentHealth, AgentId, AgentInfo, AgentType};
use crate::error::{AgentError, Result};

#[derive(Debug)]
pub struct AgentManager {
    agents: Arc<RwLock<HashMap<AgentId, Box<dyn Agent>>>>,
    agent_info: Arc<RwLock<HashMap<AgentId, AgentInfo>>>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            agent_info: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_agent(&self, mut agent: Box<dyn Agent>) -> Result<AgentId> {
        let agent_id = agent.id();
        let agent_type = agent.agent_type();
        let name = agent.name().to_string();

        info!("Registering agent: {} ({})", name, agent_type);

        agent.initialize().await.map_err(|e| {
            AgentError::Internal(anyhow::anyhow!("Failed to initialize agent {}: {}", name, e))
        })?;

        let info = AgentInfo::new(agent.as_ref());

        {
            let mut agents = self.agents.write().await;
            let mut agent_info = self.agent_info.write().await;

            if agents.contains_key(&agent_id) {
                return Err(AgentError::Internal(anyhow::anyhow!(
                    "Agent with ID {} already registered",
                    agent_id
                )));
            }

            agents.insert(agent_id, agent);
            agent_info.insert(agent_id, info);
        }

        debug!("Successfully registered agent: {} ({})", name, agent_id);
        Ok(agent_id)
    }

    pub async fn unregister_agent(&self, agent_id: AgentId) -> Result<()> {
        info!("Unregistering agent: {}", agent_id);

        let mut agents = self.agents.write().await;
        let mut agent_info = self.agent_info.write().await;

        if let Some(mut agent) = agents.remove(&agent_id) {
            if let Err(e) = agent.shutdown().await {
                warn!("Error shutting down agent {}: {}", agent_id, e);
            }
            agent_info.remove(&agent_id);
            debug!("Successfully unregistered agent: {}", agent_id);
            Ok(())
        } else {
            Err(AgentError::AgentNotFound(agent_id.to_string()))
        }
    }

    pub async fn get_agent_info(&self, agent_id: AgentId) -> Result<AgentInfo> {
        let agent_info = self.agent_info.read().await;
        agent_info
            .get(&agent_id)
            .cloned()
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))
    }

    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agent_info = self.agent_info.read().await;
        agent_info.values().cloned().collect()
    }

    pub async fn find_agents_by_type(&self, agent_type: AgentType) -> Vec<AgentInfo> {
        let agent_info = self.agent_info.read().await;
        agent_info
            .values()
            .filter(|info| info.agent_type == agent_type)
            .cloned()
            .collect()
    }

    pub async fn find_agents_by_capability(&self, capability: &str) -> Vec<AgentInfo> {
        let agent_info = self.agent_info.read().await;
        agent_info
            .values()
            .filter(|info| info.capabilities.has(capability))
            .cloned()
            .collect()
    }

    pub async fn get_agent(&self, agent_id: AgentId) -> Result<Arc<RwLock<Box<dyn Agent>>>> {
        let agents = self.agents.read().await;
        if agents.contains_key(&agent_id) {
            Err(AgentError::Internal(anyhow::anyhow!(
                "Direct agent access not implemented - use execute_task instead"
            )))
        } else {
            Err(AgentError::AgentNotFound(agent_id.to_string()))
        }
    }

    pub async fn execute_task(
        &self,
        agent_id: AgentId,
        task: crate::task::Task,
    ) -> Result<crate::task::TaskResult> {
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(&agent_id) {
            agent.execute(task).await
        } else {
            Err(AgentError::AgentNotFound(agent_id.to_string()))
        }
    }

    pub async fn health_check_all(&self) -> HashMap<AgentId, AgentHealth> {
        let agents = self.agents.read().await;
        let mut results = HashMap::new();

        for (agent_id, agent) in agents.iter() {
            match agent.health_check().await {
                Ok(health) => {
                    results.insert(*agent_id, health);
                }
                Err(e) => {
                    error!("Health check failed for agent {}: {}", agent_id, e);
                    results.insert(*agent_id, AgentHealth::Unhealthy(e.to_string()));
                }
            }
        }

        if let Ok(mut agent_info) = self.agent_info.try_write() {
            for (agent_id, health) in &results {
                if let Some(info) = agent_info.get_mut(agent_id) {
                    info.health = health.clone();
                }
            }
        }

        results
    }

    pub async fn agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    pub async fn has_agent(&self, agent_id: AgentId) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(&agent_id)
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentType;
    use crate::capability::{Capability, CapabilitySet};
    use crate::task::{Task, TaskResult};
    use async_trait::async_trait;
    use uuid::Uuid;

    struct TestAgent {
        id: AgentId,
        name: String,
        capabilities: CapabilitySet,
    }

    impl TestAgent {
        fn new(name: &str) -> Self {
            let mut capabilities = CapabilitySet::new();
            capabilities.add(Capability::new("test_capability"));

            Self {
                id: Uuid::new_v4(),
                name: name.to_string(),
                capabilities,
            }
        }
    }

    #[async_trait]
    impl Agent for TestAgent {
        fn id(&self) -> AgentId {
            self.id
        }

        fn agent_type(&self) -> AgentType {
            AgentType::Custom("test".to_string())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "Test agent for unit tests"
        }

        fn capabilities(&self) -> &CapabilitySet {
            &self.capabilities
        }

        async fn execute(&self, task: Task) -> Result<TaskResult> {
            Ok(TaskResult::success(task.id, None))
        }
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let manager = AgentManager::new();
        let agent = Box::new(TestAgent::new("test_agent"));
        let agent_id = agent.id();

        let result = manager.register_agent(agent).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), agent_id);

        assert!(manager.has_agent(agent_id).await);
        assert_eq!(manager.agent_count().await, 1);
    }

    #[tokio::test]
    async fn test_agent_unregistration() {
        let manager = AgentManager::new();
        let agent = Box::new(TestAgent::new("test_agent"));
        let agent_id = agent.id();

        manager.register_agent(agent).await.unwrap();
        assert!(manager.has_agent(agent_id).await);

        let result = manager.unregister_agent(agent_id).await;
        assert!(result.is_ok());
        assert!(!manager.has_agent(agent_id).await);
        assert_eq!(manager.agent_count().await, 0);
    }
}
