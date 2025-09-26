use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub tag: Option<String>,
    pub cmd: String,
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub execution_time: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub data: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CommandHandle {
    pub pid: u32,
    stdout_receiver: Option<tokio::sync::mpsc::Receiver<CommandOutput>>,
    stderr_receiver: Option<tokio::sync::mpsc::Receiver<CommandOutput>>,
}

impl CommandHandle {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            stdout_receiver: None,
            stderr_receiver: None,
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

#[derive(Debug, Clone)]
pub struct CommandOptions {
    pub envs: Option<HashMap<String, String>>,
    pub cwd: Option<String>,
    pub timeout: Option<std::time::Duration>,
    pub background: bool,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            envs: None,
            cwd: None,
            timeout: Some(std::time::Duration::from_secs(60)),
            background: false,
        }
    }
}