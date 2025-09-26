use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RpcMessage {
    #[serde(rename = "process_start")]
    ProcessStart {
        id: String,
        config: ProcessConfig,
    },
    #[serde(rename = "process_output")]
    ProcessOutput {
        pid: u32,
        output: ProcessOutputData,
    },
    #[serde(rename = "process_exit")]
    ProcessExit {
        pid: u32,
        exit_code: i32,
    },
    #[serde(rename = "filesystem_event")]
    FilesystemEvent {
        path: String,
        event_type: String,
        timestamp: String,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
        code: Option<i32>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub cmd: String,
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stream")]
pub enum ProcessOutputData {
    #[serde(rename = "stdout")]
    Stdout { data: String },
    #[serde(rename = "stderr")]
    Stderr { data: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}