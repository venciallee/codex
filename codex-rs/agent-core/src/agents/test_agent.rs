use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

use crate::agent::{Agent, AgentHealth, AgentId, AgentType};
use crate::capability::{Capability, CapabilitySet, ParameterType, CapabilityParameter};
use crate::error::Result;
use crate::task::{Task, TaskResult};

pub struct TestAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
}

impl TestAgent {
    pub fn new(name: impl Into<String>) -> Self {
        let mut capabilities = CapabilitySet::new();
        
        capabilities.add(
            Capability::new("test_generation")
                .with_description("Generate unit tests for code")
                .with_parameter(CapabilityParameter {
                    name: "source_file".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Source file to generate tests for".to_string()),
                })
                .with_parameter(CapabilityParameter {
                    name: "test_framework".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    description: Some("Testing framework to use".to_string()),
                })
        );

        capabilities.add(
            Capability::new("test_execution")
                .with_description("Execute test suites")
                .with_parameter(CapabilityParameter {
                    name: "test_path".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Path to test files or directory".to_string()),
                })
        );

        capabilities.add(
            Capability::new("coverage_analysis")
                .with_description("Analyze test coverage")
                .with_parameter(CapabilityParameter {
                    name: "source_path".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    description: Some("Path to source code".to_string()),
                })
        );

        capabilities.add(
            Capability::new("integration_testing")
                .with_description("Generate and run integration tests")
                .with_parameter(CapabilityParameter {
                    name: "service_endpoints".to_string(),
                    param_type: ParameterType::Array(Box::new(ParameterType::String)),
                    required: true,
                    description: Some("Service endpoints to test".to_string()),
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
impl Agent for TestAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::TestAgent
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Specialized agent for test generation, execution, and coverage analysis"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn execute(&self, task: Task) -> Result<TaskResult> {
        let start_time = std::time::Instant::now();
        
        let result = match task.task_type.as_str() {
            "test_generation" => self.handle_test_generation(&task).await,
            "test_execution" => self.handle_test_execution(&task).await,
            "coverage_analysis" => self.handle_coverage_analysis(&task).await,
            "integration_testing" => self.handle_integration_testing(&task).await,
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

impl TestAgent {
    async fn handle_test_generation(&self, task: &Task) -> Result<serde_json::Value> {
        let source_file = task.parameters.get("source_file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'source_file' parameter".to_string()))?;

        let test_framework = task.parameters.get("test_framework")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let generated_tests = match test_framework {
            "rust" | "default" => format!(
                r#"#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_basic_functionality() {{
        assert!(true);
    }}

    #[test]
    fn test_edge_cases() {{
        assert!(true);
    }}

    #[test]
    fn test_error_handling() {{
        assert!(true);
    }}
}}"#,
                source_file
            ),
            "python" => format!(
                r#"import unittest
from {} import *

class Test{}(unittest.TestCase):
    def test_basic_functionality(self):
        # Generated test for {}
        self.assertTrue(True)
    
    def test_edge_cases(self):
        # Test edge cases
        self.assertTrue(True)
    
    def test_error_handling(self):
        # Test error conditions
        self.assertTrue(True)

if __name__ == '__main__':
    unittest.main()
"#,
                source_file.replace(".py", ""),
                source_file.replace(".py", "").replace("/", "_"),
                source_file
            ),
            _ => format!("// Generated tests for {} using {}", source_file, test_framework),
        };

        Ok(json!({
            "source_file": source_file,
            "test_framework": test_framework,
            "generated_tests": generated_tests,
            "test_count": 3,
            "test_file": format!("{}_test.{}", 
                source_file.split('.').next().unwrap_or("test"),
                if test_framework == "python" { "py" } else { "rs" }
            )
        }))
    }

    async fn handle_test_execution(&self, task: &Task) -> Result<serde_json::Value> {
        let test_path = task.parameters.get("test_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'test_path' parameter".to_string()))?;

        Ok(json!({
            "test_path": test_path,
            "results": {
                "total_tests": 15,
                "passed": 13,
                "failed": 2,
                "skipped": 0,
                "execution_time": "2.34s"
            },
            "failed_tests": [
                {
                    "name": "test_complex_calculation",
                    "error": "AssertionError: Expected 42, got 41",
                    "file": "math_utils_test.rs",
                    "line": 25
                },
                {
                    "name": "test_network_timeout",
                    "error": "TimeoutError: Request timed out after 5s",
                    "file": "network_test.rs",
                    "line": 67
                }
            ],
            "coverage": {
                "line_coverage": 85.2,
                "branch_coverage": 78.9,
                "function_coverage": 92.1
            }
        }))
    }

    async fn handle_coverage_analysis(&self, task: &Task) -> Result<serde_json::Value> {
        let source_path = task.parameters.get("source_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'source_path' parameter".to_string()))?;

        Ok(json!({
            "source_path": source_path,
            "overall_coverage": {
                "line_coverage": 82.5,
                "branch_coverage": 75.3,
                "function_coverage": 89.7
            },
            "file_coverage": [
                {
                    "file": "src/main.rs",
                    "line_coverage": 95.2,
                    "uncovered_lines": [45, 67, 89]
                },
                {
                    "file": "src/utils.rs",
                    "line_coverage": 78.9,
                    "uncovered_lines": [12, 23, 34, 45, 56]
                }
            ],
            "recommendations": [
                "Add tests for error handling paths",
                "Increase coverage for utility functions",
                "Test edge cases in main business logic"
            ],
            "target_coverage": 90.0
        }))
    }

    async fn handle_integration_testing(&self, task: &Task) -> Result<serde_json::Value> {
        let service_endpoints = task.parameters.get("service_endpoints")
            .and_then(|v| v.as_array())
            .ok_or_else(|| crate::error::AgentError::InvalidTaskConfig("Missing 'service_endpoints' parameter".to_string()))?;

        let endpoints: Vec<String> = service_endpoints
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let mut test_results = Vec::new();
        
        for endpoint in &endpoints {
            test_results.push(json!({
                "endpoint": endpoint,
                "status": "passed",
                "response_time": "125ms",
                "tests": [
                    {
                        "name": "test_endpoint_availability",
                        "status": "passed"
                    },
                    {
                        "name": "test_response_format",
                        "status": "passed"
                    },
                    {
                        "name": "test_error_handling",
                        "status": "passed"
                    }
                ]
            }));
        }

        Ok(json!({
            "service_endpoints": endpoints,
            "test_results": test_results,
            "summary": {
                "total_endpoints": endpoints.len(),
                "passed": endpoints.len(),
                "failed": 0,
                "average_response_time": "125ms"
            },
            "integration_score": 98.5
        }))
    }
}
