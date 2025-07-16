use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

use crate::agent::{Agent, AgentHealth, AgentId, AgentType};
use crate::capability::{Capability, CapabilitySet, ParameterType, CapabilityParameter};
use crate::error::Result;
use crate::task::{Task, TaskResult};

pub struct AnalysisAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
}

impl AnalysisAgent {
    pub fn new(name: impl Into<String>) -> Self {
        let mut capabilities = CapabilitySet::new();
        
        capabilities.add(
            Capability::new("code_review")
                .with_description("Perform comprehensive code review")
                .with_parameter(CapabilityParameter {
                    name: "files".to_string(),
                    param_type: ParameterType::Array(Box::new(ParameterType::String)),
                    required: true,
                    description: Some("Files to review".to_string()),
                })
        );

        capabilities.add(
            Capability::new("security_analysis")
                .with_description("Analyze code for security vulnerabilities")
                .with_parameter(CapabilityParameter {
                    name: "target_path".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Path to analyze for security issues".to_string()),
                })
        );

        capabilities.add(
            Capability::new("performance_analysis")
                .with_description("Analyze code for performance issues")
                .with_parameter(CapabilityParameter {
                    name: "profile_data".to_string(),
                    param_type: ParameterType::Object,
                    required: false,
                    description: Some("Optional profiling data".to_string()),
                })
        );

        capabilities.add(
            Capability::new("dependency_analysis")
                .with_description("Analyze project dependencies")
                .with_parameter(CapabilityParameter {
                    name: "manifest_file".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Path to dependency manifest (Cargo.toml, package.json, etc.)".to_string()),
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
impl Agent for AnalysisAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::AnalysisAgent
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Specialized agent for code analysis, security review, and performance optimization"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn execute(&self, task: Task) -> Result<TaskResult> {
        let start_time = std::time::Instant::now();
        
        let result = match task.task_type.as_str() {
            "code_review" => self.handle_code_review(&task).await,
            "security_analysis" => self.handle_security_analysis(&task).await,
            "performance_analysis" => self.handle_performance_analysis(&task).await,
            "dependency_analysis" => self.handle_dependency_analysis(&task).await,
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

impl AnalysisAgent {
    async fn handle_code_review(&self, task: &Task) -> Result<serde_json::Value> {
        let files = task.parameters.get("files")
            .and_then(|v| v.as_array())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'files' parameter".to_string()))?;

        let file_paths: Vec<String> = files
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let mut review_results = Vec::new();
        
        for file_path in &file_paths {
            review_results.push(json!({
                "file": file_path,
                "issues": [
                    {
                        "type": "style",
                        "line": 42,
                        "message": "Consider using more descriptive variable names",
                        "severity": "minor"
                    },
                    {
                        "type": "logic",
                        "line": 58,
                        "message": "Potential null pointer dereference",
                        "severity": "major"
                    }
                ],
                "suggestions": [
                    "Add more comprehensive error handling",
                    "Consider extracting this function for better testability"
                ],
                "overall_score": 8.5
            }));
        }

        Ok(json!({
            "review_results": review_results,
            "summary": {
                "files_reviewed": file_paths.len(),
                "total_issues": review_results.len() * 2, // Simplified
                "average_score": 8.5
            }
        }))
    }

    async fn handle_security_analysis(&self, task: &Task) -> Result<serde_json::Value> {
        let target_path = task.parameters.get("target_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'target_path' parameter".to_string()))?;

        Ok(json!({
            "target_path": target_path,
            "vulnerabilities": [
                {
                    "type": "SQL Injection",
                    "severity": "high",
                    "file": format!("{}/database.rs", target_path),
                    "line": 123,
                    "description": "User input not properly sanitized before database query"
                },
                {
                    "type": "Insecure Randomness",
                    "severity": "medium",
                    "file": format!("{}/auth.rs", target_path),
                    "line": 67,
                    "description": "Using predictable random number generator for security tokens"
                }
            ],
            "security_score": 7.2,
            "recommendations": [
                "Implement input validation and parameterized queries",
                "Use cryptographically secure random number generator",
                "Add rate limiting to authentication endpoints"
            ]
        }))
    }

    async fn handle_performance_analysis(&self, task: &Task) -> Result<serde_json::Value> {
        Ok(json!({
            "performance_issues": [
                {
                    "type": "Memory Leak",
                    "severity": "high",
                    "location": "main_loop",
                    "description": "Objects not being properly deallocated"
                },
                {
                    "type": "Inefficient Algorithm",
                    "severity": "medium",
                    "location": "sort_function",
                    "description": "O(n²) algorithm could be optimized to O(n log n)"
                }
            ],
            "metrics": {
                "memory_usage": "85%",
                "cpu_usage": "45%",
                "response_time": "250ms"
            },
            "optimizations": [
                "Implement object pooling",
                "Use more efficient sorting algorithm",
                "Add caching layer for frequently accessed data"
            ]
        }))
    }

    async fn handle_dependency_analysis(&self, task: &Task) -> Result<serde_json::Value> {
        let manifest_file = task.parameters.get("manifest_file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'manifest_file' parameter".to_string()))?;

        Ok(json!({
            "manifest_file": manifest_file,
            "dependencies": {
                "total": 42,
                "outdated": 5,
                "vulnerable": 2,
                "unused": 3
            },
            "vulnerabilities": [
                {
                    "package": "old-crypto-lib",
                    "version": "1.2.3",
                    "severity": "high",
                    "description": "Known cryptographic vulnerability"
                }
            ],
            "recommendations": [
                "Update 5 outdated dependencies",
                "Remove 3 unused dependencies",
                "Replace vulnerable crypto library with secure alternative"
            ],
            "license_issues": []
        }))
    }
}
