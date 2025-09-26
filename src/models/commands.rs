use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

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
    stdout: Option<mpsc::Receiver<CommandOutput>>,
    stderr: Option<mpsc::Receiver<CommandOutput>>,
    result: Option<oneshot::Receiver<CommandResult>>,
}

impl CommandHandle {
    pub fn new(
        pid: u32,
        stdout: mpsc::Receiver<CommandOutput>,
        stderr: mpsc::Receiver<CommandOutput>,
        result: oneshot::Receiver<CommandResult>,
    ) -> Self {
        Self {
            pid,
            stdout: Some(stdout),
            stderr: Some(stderr),
            result: Some(result),
        }
    }

    pub fn from_pid(pid: u32) -> Self {
        Self {
            pid,
            stdout: None,
            stderr: None,
            result: None,
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn take_stdout(&mut self) -> Option<mpsc::Receiver<CommandOutput>> {
        self.stdout.take()
    }

    pub fn take_stderr(&mut self) -> Option<mpsc::Receiver<CommandOutput>> {
        self.stderr.take()
    }

    pub fn take_result(&mut self) -> Option<oneshot::Receiver<CommandResult>> {
        self.result.take()
    }

    pub fn on_stdout<F>(&mut self, mut callback: F)
    where
        F: FnMut(CommandOutput) + Send + 'static,
    {
        if let Some(mut rx) = self.stdout.take() {
            tokio::spawn(async move {
                while let Some(item) = rx.recv().await {
                    callback(item);
                }
            });
        }
    }

    pub fn on_stderr<F>(&mut self, mut callback: F)
    where
        F: FnMut(CommandOutput) + Send + 'static,
    {
        if let Some(mut rx) = self.stderr.take() {
            tokio::spawn(async move {
                while let Some(item) = rx.recv().await {
                    callback(item);
                }
            });
        }
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
