use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::agent::{AgentId, AgentType};
use crate::error::{AgentError, Result};
use crate::manager::AgentManager;
use crate::scheduler::{TaskScheduler, SchedulerStats};
use crate::task::{Task, TaskId, TaskResult, TaskStatus};

#[derive(Debug)]
pub struct Coordinator {
    agent_manager: Arc<AgentManager>,
    scheduler: Arc<TaskScheduler>,
    execution_strategy: ExecutionStrategy,
    active_executions: Arc<RwLock<HashMap<TaskId, AgentId>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStrategy {
    Sequential,
    Parallel { max_concurrent: usize },
    Pipeline,
    Collaborative,
}

impl Default for ExecutionStrategy {
    fn default() -> Self {
        ExecutionStrategy::Parallel { max_concurrent: 4 }
    }
}

impl Coordinator {
    pub fn new(agent_manager: Arc<AgentManager>, scheduler: Arc<TaskScheduler>) -> Self {
        Self {
            agent_manager,
            scheduler,
            execution_strategy: ExecutionStrategy::default(),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_execution_strategy(mut self, strategy: ExecutionStrategy) -> Self {
        self.execution_strategy = strategy;
        self
    }

    pub async fn submit_task(&self, task: Task) -> Result<TaskId> {
        let task_id = task.id;
        info!("Submitting task: {} ({})", task.description, task_id);

        self.validate_task_requirements(&task).await?;

        self.scheduler.schedule_task(task).await?;

        self.try_execute_pending_tasks().await?;

        Ok(task_id)
    }

    pub async fn submit_task_batch(&self, tasks: Vec<Task>) -> Result<Vec<TaskId>> {
        let mut task_ids = Vec::new();

        for task in tasks {
            let task_id = self.submit_task(task).await?;
            task_ids.push(task_id);
        }

        Ok(task_ids)
    }

    pub async fn get_task_status(&self, task_id: TaskId) -> Result<TaskExecutionStatus> {
        let active_executions = self.active_executions.read().await;
        
        if let Some(agent_id) = active_executions.get(&task_id) {
            return Ok(TaskExecutionStatus::Running { agent_id: *agent_id });
        }

        let stats = self.scheduler.get_stats().await;
        let running_tasks = self.scheduler.get_running_tasks().await;
        
        for (running_task_id, agent_id, duration) in running_tasks {
            if running_task_id == task_id {
                return Ok(TaskExecutionStatus::Running { agent_id });
            }
        }

        Ok(TaskExecutionStatus::Unknown)
    }

    pub async fn cancel_task(&self, task_id: TaskId) -> Result<bool> {
        info!("Attempting to cancel task: {}", task_id);

        let cancelled = self.scheduler.cancel_task(task_id).await?;
        
        if cancelled {
            info!("Successfully cancelled pending task: {}", task_id);
            return Ok(true);
        }

        let active_executions = self.active_executions.read().await;
        if active_executions.contains_key(&task_id) {
            warn!("Cannot cancel running task: {}", task_id);
            return Ok(false);
        }

        Ok(false)
    }

    pub async fn get_system_stats(&self) -> SystemStats {
        let scheduler_stats = self.scheduler.get_stats().await;
        let agent_count = self.agent_manager.agent_count().await;
        let active_executions = self.active_executions.read().await;
        let active_execution_count = active_executions.len();

        SystemStats {
            total_agents: agent_count,
            active_executions: active_execution_count,
            scheduler_stats,
        }
    }

    pub async fn health_check(&self) -> HealthCheckResult {
        let agent_health = self.agent_manager.health_check_all().await;
        let unhealthy_agents = agent_health
            .iter()
            .filter(|(_, health)| !health.is_healthy())
            .count();

        let system_stats = self.get_system_stats().await;

        HealthCheckResult {
            overall_healthy: unhealthy_agents == 0,
            total_agents: agent_health.len(),
            healthy_agents: agent_health.len() - unhealthy_agents,
            unhealthy_agents,
            system_stats,
        }
    }

    async fn try_execute_pending_tasks(&self) -> Result<()> {
        match &self.execution_strategy {
            ExecutionStrategy::Sequential => {
                self.execute_sequential().await
            }
            ExecutionStrategy::Parallel { max_concurrent } => {
                self.execute_parallel(*max_concurrent).await
            }
            ExecutionStrategy::Pipeline => {
                self.execute_pipeline().await
            }
            ExecutionStrategy::Collaborative => {
                self.execute_collaborative().await
            }
        }
    }

    async fn execute_sequential(&self) -> Result<()> {
        let active_executions = self.active_executions.read().await;
        if !active_executions.is_empty() {
            return Ok(());
        }
        drop(active_executions);

        if let Some(task) = self.scheduler.get_next_task().await {
            self.execute_single_task(task).await?;
        }

        Ok(())
    }

    async fn execute_parallel(&self, max_concurrent: usize) -> Result<()> {
        let active_count = {
            let active_executions = self.active_executions.read().await;
            active_executions.len()
        };

        let available_slots = max_concurrent.saturating_sub(active_count);
        
        for _ in 0..available_slots {
            if let Some(task) = self.scheduler.get_next_task().await {
                self.execute_single_task(task).await?;
            } else {
                break;
            }
        }

        Ok(())
    }

    async fn execute_pipeline(&self) -> Result<()> {
        self.execute_parallel(4).await
    }

    async fn execute_collaborative(&self) -> Result<()> {
        self.execute_parallel(8).await
    }

    async fn execute_single_task(&self, task: Task) -> Result<()> {
        let task_id = task.id;
        
        let agent_id = self.find_suitable_agent(&task).await?;
        
        debug!("Executing task {} on agent {}", task_id, agent_id);

        self.scheduler.mark_task_running(task.clone(), agent_id).await?;
        
        {
            let mut active_executions = self.active_executions.write().await;
            active_executions.insert(task_id, agent_id);
        }

        let agent_manager = Arc::clone(&self.agent_manager);
        let scheduler = Arc::clone(&self.scheduler);
        let active_executions = Arc::clone(&self.active_executions);
        
        tokio::spawn(async move {
            let result = agent_manager.execute_task(agent_id, task).await;
            
            {
                let mut active = active_executions.write().await;
                active.remove(&task_id);
            }

            match result {
                Ok(task_result) => {
                    if task_result.is_success() {
                        if let Err(e) = scheduler.mark_task_completed(task_id).await {
                            error!("Failed to mark task {} as completed: {}", task_id, e);
                        }
                    } else {
                        let error_msg = task_result.error.unwrap_or_else(|| "Unknown error".to_string());
                        if let Err(e) = scheduler.mark_task_failed(task_id, &error_msg).await {
                            error!("Failed to mark task {} as failed: {}", task_id, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Task {} execution failed: {}", task_id, e);
                    if let Err(e) = scheduler.mark_task_failed(task_id, &e.to_string()).await {
                        error!("Failed to mark task {} as failed: {}", task_id, e);
                    }
                }
            }
        });

        Ok(())
    }

    async fn find_suitable_agent(&self, task: &Task) -> Result<AgentId> {
        let agents = self.agent_manager.list_agents().await;
        
        let suitable_agents: Vec<_> = agents
            .into_iter()
            .filter(|agent_info| {
                agent_info.health.is_healthy() && 
                agent_info.capabilities.matches_requirements(&task.required_capabilities)
            })
            .collect();

        if suitable_agents.is_empty() {
            return Err(AgentError::CapabilityMismatch {
                required: task.required_capabilities.join(", "),
                available: vec![], // Could be improved to show available capabilities
            });
        }

        Ok(suitable_agents[0].id)
    }

    async fn validate_task_requirements(&self, task: &Task) -> Result<()> {
        if task.required_capabilities.is_empty() {
            return Ok(());
        }

        let agents = self.agent_manager.list_agents().await;
        let has_capable_agent = agents.iter().any(|agent_info| {
            agent_info.capabilities.matches_requirements(&task.required_capabilities)
        });

        if !has_capable_agent {
            return Err(AgentError::CapabilityMismatch {
                required: task.required_capabilities.join(", "),
                available: agents
                    .iter()
                    .flat_map(|a| a.capabilities.iter().map(|c| c.name.clone()))
                    .collect(),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum TaskExecutionStatus {
    Pending,
    Running { agent_id: AgentId },
    Completed,
    Failed(String),
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub total_agents: usize,
    pub active_executions: usize,
    pub scheduler_stats: SchedulerStats,
}

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub overall_healthy: bool,
    pub total_agents: usize,
    pub healthy_agents: usize,
    pub unhealthy_agents: usize,
    pub system_stats: SystemStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Agent, AgentHealth, AgentType};
    use crate::capability::{Capability, CapabilitySet};
    use crate::task::Task;
    use async_trait::async_trait;
    use uuid::Uuid;

    struct TestAgent {
        id: AgentId,
        capabilities: CapabilitySet,
    }

    impl TestAgent {
        fn new_with_capability(capability: &str) -> Self {
            let mut capabilities = CapabilitySet::new();
            capabilities.add(Capability::new(capability));
            
            Self {
                id: Uuid::new_v4(),
                capabilities,
            }
        }
    }

    #[async_trait]
    impl Agent for TestAgent {
        fn id(&self) -> AgentId { self.id }
        fn agent_type(&self) -> AgentType { AgentType::Custom("test".to_string()) }
        fn name(&self) -> &str { "test_agent" }
        fn description(&self) -> &str { "Test agent" }
        fn capabilities(&self) -> &CapabilitySet { &self.capabilities }

        async fn execute(&self, task: Task) -> Result<TaskResult> {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(TaskResult::success(task.id, None))
        }
    }

    #[tokio::test]
    async fn test_coordinator_task_submission() {
        let agent_manager = Arc::new(AgentManager::new());
        let scheduler = Arc::new(TaskScheduler::new());
        let coordinator = Coordinator::new(agent_manager.clone(), scheduler);

        let agent = Box::new(TestAgent::new_with_capability("test_capability"));
        agent_manager.register_agent(agent).await.unwrap();

        let task = Task::new("test", "Test task")
            .with_capability("test_capability");
        
        let task_id = coordinator.submit_task(task).await.unwrap();
        assert!(!task_id.is_nil());

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let stats = coordinator.get_system_stats().await;
        assert_eq!(stats.total_agents, 1);
    }
}
