use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionRequest {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub stdout: String,
    pub stderr: String,
    pub results: Vec<Result>,
    pub error: Option<ExecutionError>,
    pub is_main_result: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    #[serde(rename = "type")]
    pub result_type: String,
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub name: String,
    pub value: String,
    pub traceback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMessage {
    pub line: String,
    pub timestamp: i64,
    pub error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub id: String,
    pub language: String,
    pub cwd: String,
}

impl Context {
    pub fn new(id: String, language: String, cwd: String) -> Self {
        Self { id, language, cwd }
    }
}

#[derive(Debug, Clone)]
pub struct CodeInterpreterOptions {
    pub language: Option<String>,
    pub context: Option<Context>,
    pub env_vars: Option<HashMap<String, String>>,
    pub timeout: Option<std::time::Duration>,
}

impl Default for CodeInterpreterOptions {
    fn default() -> Self {
        Self {
            language: Some("python".to_string()),
            context: None,
            env_vars: None,
            timeout: Some(std::time::Duration::from_secs(300)),
        }
    }
}