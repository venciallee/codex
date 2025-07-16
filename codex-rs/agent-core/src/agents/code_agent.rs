use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::agent::{Agent, AgentHealth, AgentId, AgentType};
use crate::capability::{Capability, CapabilitySet, ParameterType, CapabilityParameter};
use crate::error::Result;
use crate::task::{Task, TaskResult};

pub struct CodeAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
}

impl CodeAgent {
    pub fn new(name: impl Into<String>) -> Self {
        let mut capabilities = CapabilitySet::new();
        
        capabilities.add(
            Capability::new("code_generation")
                .with_description("Generate code based on specifications")
                .with_parameter(CapabilityParameter {
                    name: "language".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Programming language to generate code in".to_string()),
                })
                .with_parameter(CapabilityParameter {
                    name: "specification".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Code specification or requirements".to_string()),
                })
        );

        capabilities.add(
            Capability::new("code_modification")
                .with_description("Modify existing code")
                .with_parameter(CapabilityParameter {
                    name: "file_path".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Path to the file to modify".to_string()),
                })
                .with_parameter(CapabilityParameter {
                    name: "changes".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Description of changes to make".to_string()),
                })
        );

        capabilities.add(
            Capability::new("code_refactoring")
                .with_description("Refactor code to improve structure and maintainability")
                .with_parameter(CapabilityParameter {
                    name: "target_files".to_string(),
                    param_type: ParameterType::Array(Box::new(ParameterType::String)),
                    required: true,
                    description: Some("Files to refactor".to_string()),
                })
        );

        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            capabilities,
        }
    }
}

#[async_trait]
impl Agent for CodeAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::CodeAgent
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Specialized agent for code generation, modification, and refactoring"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn execute(&self, task: Task) -> Result<TaskResult> {
        let start_time = std::time::Instant::now();
        
        let result = match task.task_type.as_str() {
            "code_generation" => self.handle_code_generation(&task).await,
            "code_modification" => self.handle_code_modification(&task).await,
            "code_refactoring" => self.handle_code_refactoring(&task).await,
            _ => {
                return Ok(TaskResult::failure(
                    task.id,
                    format!("Unsupported task type: {}", task.task_type)
                ));
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(output) => Ok(TaskResult::success(task.id, Some(output)).with_execution_time(execution_time)),
            Err(e) => Ok(TaskResult::failure(task.id, e.to_string()).with_execution_time(execution_time)),
        }
    }

    async fn health_check(&self) -> Result<AgentHealth> {
        
        Ok(AgentHealth::Healthy)
    }
}

impl CodeAgent {
    async fn handle_code_generation(&self, task: &Task) -> Result<serde_json::Value> {
        let language = task.parameters.get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'language' parameter".to_string()))?;

        let specification = task.parameters.get("specification")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'specification' parameter".to_string()))?;

        
        let generated_code = match language {
            "rust" => format!("// Generated Rust code for: {}\nfn main() {{\n    println!(\"Hello, world!\");\n}}", specification),
            "python" => format!("# Generated Python code for: {}\ndef main():\n    print(\"Hello, world!\")", specification),
            "javascript" => format!("// Generated JavaScript code for: {}\nconsole.log(\"Hello, world!\");", specification),
            _ => format!("// Generated code for: {} (language: {})\n// Code generation not implemented for this language", specification, language),
        };

        Ok(json!({
            "generated_code": generated_code,
            "language": language,
            "specification": specification,
            "file_extension": match language {
                "rust" => "rs",
                "python" => "py",
                "javascript" => "js",
                _ => "txt"
            }
        }))
    }

    async fn handle_code_modification(&self, task: &Task) -> Result<serde_json::Value> {
        let file_path = task.parameters.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'file_path' parameter".to_string()))?;

        let changes = task.parameters.get("changes")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'changes' parameter".to_string()))?;


        Ok(json!({
            "file_path": file_path,
            "changes_applied": changes,
            "status": "modified",
            "diff": format!("+ // Modified: {}", changes)
        }))
    }

    async fn handle_code_refactoring(&self, task: &Task) -> Result<serde_json::Value> {
        let target_files = task.parameters.get("target_files")
            .and_then(|v| v.as_array())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'target_files' parameter".to_string()))?;

        let file_paths: Vec<String> = target_files
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();


        let mut refactored_files = HashMap::new();
        for file_path in &file_paths {
            refactored_files.insert(file_path.clone(), format!("// Refactored: {}", file_path));
        }

        Ok(json!({
            "refactored_files": refactored_files,
            "refactoring_summary": format!("Refactored {} files", file_paths.len()),
            "improvements": [
                "Improved code structure",
                "Enhanced readability",
                "Reduced complexity"
            ]
        }))
    }
}
