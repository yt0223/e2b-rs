use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_team_id() -> String {
    "default".to_string()
}

fn default_true() -> bool {
    true
}

fn default_datetime() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sandbox {
    #[serde(alias = "sandboxID")]
    pub sandbox_id: String,
    #[serde(alias = "templateID")]
    pub template_id: String,
    pub alias: Option<String>,
    #[serde(alias = "clientID")]
    pub client_id: String,
    #[serde(alias = "teamID", default = "default_team_id")]
    pub team_id: String,
    pub name: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub start_cmd: Option<String>,
    pub cwd: Option<String>,
    #[serde(alias = "envVars")]
    pub env_vars: Option<HashMap<String, String>>,
    #[serde(alias = "cpuCount", default)]
    pub cpu_count: u32,
    #[serde(alias = "memoryMB", default)]
    pub memory_mb: u32,
    #[serde(alias = "isLive", default = "default_true")]
    pub is_live: bool,
    #[serde(alias = "createdAt", default = "default_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(alias = "updatedAt", default = "default_datetime")]
    pub updated_at: DateTime<Utc>,
    #[serde(alias = "pausedAt")]
    pub paused_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxCreateRequest {
    #[serde(rename = "templateID")]
    pub template_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "autoPause")]
    pub auto_pause: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_internet_access: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "envVars")]
    pub env_vars: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecution {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub error: Option<String>,
    pub results: Vec<ExecutionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    #[serde(rename = "type")]
    pub result_type: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub markdown: Option<String>,
    pub svg: Option<String>,
    pub png: Option<String>,
    pub jpeg: Option<String>,
    pub pdf: Option<String>,
    pub latex: Option<String>,
    pub json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub memory_limit_mb: u64,
    pub disk_usage_mb: u64,
    pub disk_limit_mb: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxLog {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}