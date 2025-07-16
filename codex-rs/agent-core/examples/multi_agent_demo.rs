//! 

use codex_agent_core::{
    Agent, AgentManager, AnalysisAgent, CodeAgent, Coordinator, ExecutionStrategy, Task,
    TaskScheduler, TestAgent,
};
use serde_json::json;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🚀 Multi-Agent Codex Demo");
    println!("==========================");

    let agent_manager = Arc::new(AgentManager::new());
    let scheduler = Arc::new(TaskScheduler::new());
    let coordinator = Coordinator::new(agent_manager.clone(), scheduler.clone())
        .with_execution_strategy(ExecutionStrategy::Parallel { max_concurrent: 3 });

    println!("\n📋 Registering agents...");
    
    let code_agent = Box::new(CodeAgent::new("CodeGen-1"));
    let analysis_agent = Box::new(AnalysisAgent::new("Analyzer-1"));
    let test_agent = Box::new(TestAgent::new("Tester-1"));

    let code_agent_id = agent_manager.register_agent(code_agent).await?;
    let analysis_agent_id = agent_manager.register_agent(analysis_agent).await?;
    let test_agent_id = agent_manager.register_agent(test_agent).await?;

    println!("✅ Registered CodeAgent: {}", code_agent_id);
    println!("✅ Registered AnalysisAgent: {}", analysis_agent_id);
    println!("✅ Registered TestAgent: {}", test_agent_id);

    let stats = coordinator.get_system_stats().await;
    println!("\n📊 System Stats:");
    println!("   Total agents: {}", stats.total_agents);
    println!("   Active executions: {}", stats.active_executions);

    println!("\n🎯 Submitting tasks...");

    let code_task = Task::new("code_generation", "Generate a simple calculator function")
        .with_capability("code_generation")
        .with_parameter("language", json!("rust"))
        .with_parameter("specification", json!("Create a function that adds two numbers"));

    let code_task_id = coordinator.submit_task(code_task).await?;
    println!("📝 Submitted code generation task: {}", code_task_id);

    let analysis_task = Task::new("security_analysis", "Analyze project for security vulnerabilities")
        .with_capability("security_analysis")
        .with_parameter("target_path", json!("src/"));

    let analysis_task_id = coordinator.submit_task(analysis_task).await?;
    println!("🔍 Submitted security analysis task: {}", analysis_task_id);

    let test_task = Task::new("test_generation", "Generate unit tests for calculator")
        .with_capability("test_generation")
        .with_parameter("source_file", json!("calculator.rs"))
        .with_parameter("test_framework", json!("rust"));

    let test_task_id = coordinator.submit_task(test_task).await?;
    println!("🧪 Submitted test generation task: {}", test_task_id);

    println!("\n⏳ Waiting for task execution...");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("\n📋 Task Status Report:");
    for (task_name, task_id) in [
        ("Code Generation", code_task_id),
        ("Security Analysis", analysis_task_id),
        ("Test Generation", test_task_id),
    ] {
        match coordinator.get_task_status(task_id).await {
            Ok(status) => println!("   {}: {:?}", task_name, status),
            Err(e) => println!("   {}: Error - {}", task_name, e),
        }
    }

    println!("\n🏥 Health Check:");
    let health = coordinator.health_check().await;
    println!("   Overall healthy: {}", health.overall_healthy);
    println!("   Healthy agents: {}/{}", health.healthy_agents, health.total_agents);

    let final_stats = coordinator.get_system_stats().await;
    println!("\n📊 Final System Stats:");
    println!("   Total agents: {}", final_stats.total_agents);
    println!("   Active executions: {}", final_stats.active_executions);
    println!("   Pending tasks: {}", final_stats.scheduler_stats.pending_count);
    println!("   Completed tasks: {}", final_stats.scheduler_stats.completed_count);

    println!("\n✨ Demo completed successfully!");
    Ok(())
}
