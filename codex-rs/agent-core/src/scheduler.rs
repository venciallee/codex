use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::agent::AgentId;
use crate::error::{AgentError, Result};
use crate::task::{Priority, Task, TaskId, TaskStatus};

#[derive(Debug)]
pub struct TaskScheduler {
    pending_tasks: Arc<RwLock<BinaryHeap<ScheduledTask>>>,
    running_tasks: Arc<RwLock<HashMap<TaskId, RunningTask>>>,
    completed_tasks: Arc<RwLock<HashSet<TaskId>>>,
    failed_tasks: Arc<RwLock<HashSet<TaskId>>>,
    dependencies: Arc<RwLock<HashMap<TaskId, Vec<TaskId>>>>,
}

#[derive(Debug, Clone)]
struct ScheduledTask {
    task: Task,
    scheduled_at: std::time::Instant,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.priority == other.task.priority
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.task.priority.cmp(&other.task.priority)
            .then_with(|| other.scheduled_at.cmp(&self.scheduled_at)) // Earlier tasks first for same priority
    }
}

#[derive(Debug, Clone)]
struct RunningTask {
    task: Task,
    agent_id: AgentId,
    started_at: std::time::Instant,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            pending_tasks: Arc::new(RwLock::new(BinaryHeap::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(HashSet::new())),
            failed_tasks: Arc::new(RwLock::new(HashSet::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn schedule_task(&self, task: Task) -> Result<()> {
        let task_id = task.id;
        info!("Scheduling task: {} ({})", task.description, task_id);

        if !task.dependencies.is_empty() {
            let mut dependencies = self.dependencies.write().await;
            dependencies.insert(task_id, task.dependencies.clone());
        }

        if self.are_dependencies_satisfied(&task).await? {
            let scheduled_task = ScheduledTask {
                task,
                scheduled_at: std::time::Instant::now(),
            };

            let mut pending = self.pending_tasks.write().await;
            pending.push(scheduled_task);
            debug!("Task {} added to pending queue", task_id);
        } else {
            debug!("Task {} waiting for dependencies", task_id);
        }

        Ok(())
    }

    pub async fn get_next_task(&self) -> Option<Task> {
        let mut pending = self.pending_tasks.write().await;
        
        while let Some(scheduled_task) = pending.pop() {
            let task = scheduled_task.task;
            
            if self.are_dependencies_satisfied(&task).await.unwrap_or(false) {
                debug!("Retrieved task for execution: {}", task.id);
                return Some(task);
            } else {
                pending.push(scheduled_task);
                break;
            }
        }
        
        None
    }

    pub async fn mark_task_running(&self, task: Task, agent_id: AgentId) -> Result<()> {
        let task_id = task.id;
        debug!("Marking task {} as running on agent {}", task_id, agent_id);

        let running_task = RunningTask {
            task,
            agent_id,
            started_at: std::time::Instant::now(),
        };

        let mut running = self.running_tasks.write().await;
        running.insert(task_id, running_task);

        Ok(())
    }

    pub async fn mark_task_completed(&self, task_id: TaskId) -> Result<()> {
        info!("Marking task {} as completed", task_id);

        {
            let mut running = self.running_tasks.write().await;
            running.remove(&task_id);
        }

        {
            let mut completed = self.completed_tasks.write().await;
            completed.insert(task_id);
        }

        self.check_and_schedule_dependent_tasks(task_id).await?;

        Ok(())
    }

    pub async fn mark_task_failed(&self, task_id: TaskId, _error: &str) -> Result<()> {
        warn!("Marking task {} as failed", task_id);

        {
            let mut running = self.running_tasks.write().await;
            running.remove(&task_id);
        }

        {
            let mut failed = self.failed_tasks.write().await;
            failed.insert(task_id);
        }


        Ok(())
    }

    pub async fn get_stats(&self) -> SchedulerStats {
        let pending_count = self.pending_tasks.read().await.len();
        let running_count = self.running_tasks.read().await.len();
        let completed_count = self.completed_tasks.read().await.len();
        let failed_count = self.failed_tasks.read().await.len();

        SchedulerStats {
            pending_count,
            running_count,
            completed_count,
            failed_count,
        }
    }

    pub async fn get_running_tasks(&self) -> Vec<(TaskId, AgentId, std::time::Duration)> {
        let running = self.running_tasks.read().await;
        let now = std::time::Instant::now();
        
        running
            .iter()
            .map(|(task_id, running_task)| {
                (*task_id, running_task.agent_id, now.duration_since(running_task.started_at))
            })
            .collect()
    }

    pub async fn cancel_task(&self, task_id: TaskId) -> Result<bool> {
        let mut pending = self.pending_tasks.write().await;
        let original_len = pending.len();
        
        let tasks: Vec<_> = pending.drain().collect();
        for scheduled_task in tasks {
            if scheduled_task.task.id != task_id {
                pending.push(scheduled_task);
            }
        }
        
        let was_cancelled = pending.len() < original_len;
        if was_cancelled {
            info!("Cancelled pending task: {}", task_id);
        }
        
        Ok(was_cancelled)
    }

    async fn are_dependencies_satisfied(&self, task: &Task) -> Result<bool> {
        if task.dependencies.is_empty() {
            return Ok(true);
        }

        let completed = self.completed_tasks.read().await;
        let failed = self.failed_tasks.read().await;

        for dep_id in &task.dependencies {
            if failed.contains(dep_id) {
                return Err(AgentError::SchedulingError(format!(
                    "Task {} depends on failed task {}",
                    task.id, dep_id
                )));
            }
            if !completed.contains(dep_id) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn check_and_schedule_dependent_tasks(&self, completed_task_id: TaskId) -> Result<()> {
        
        debug!("Checking for tasks dependent on {}", completed_task_id);
        
        
        Ok(())
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub pending_count: usize,
    pub running_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_task_scheduling() {
        let scheduler = TaskScheduler::new();
        let task = Task::new("test", "Test task");
        let task_id = task.id;

        scheduler.schedule_task(task).await.unwrap();

        let stats = scheduler.get_stats().await;
        assert_eq!(stats.pending_count, 1);
        assert_eq!(stats.running_count, 0);

        let next_task = scheduler.get_next_task().await;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().id, task_id);
    }

    #[tokio::test]
    async fn test_task_priority_ordering() {
        let scheduler = TaskScheduler::new();
        
        let low_task = Task::new("low", "Low priority task")
            .with_priority(Priority::Low);
        let high_task = Task::new("high", "High priority task")
            .with_priority(Priority::High);

        scheduler.schedule_task(low_task).await.unwrap();
        scheduler.schedule_task(high_task.clone()).await.unwrap();

        let next_task = scheduler.get_next_task().await;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().id, high_task.id);
    }

    #[tokio::test]
    async fn test_task_dependencies() {
        let scheduler = TaskScheduler::new();
        
        let task1 = Task::new("task1", "First task");
        let task1_id = task1.id;
        
        let task2 = Task::new("task2", "Second task")
            .with_dependency(task1_id);

        scheduler.schedule_task(task1).await.unwrap();
        scheduler.schedule_task(task2).await.unwrap();

        let next_task = scheduler.get_next_task().await;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().id, task1_id);

        let next_task = scheduler.get_next_task().await;
        assert!(next_task.is_none());

        scheduler.mark_task_completed(task1_id).await.unwrap();

        let next_task = scheduler.get_next_task().await;
        assert!(next_task.is_some());
    }
}
