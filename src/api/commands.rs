use crate::{
    error::{Error, Result},
    models::{CommandHandle, CommandOptions, CommandOutput, CommandResult, ProcessInfo},
    rpc::RpcClient,
};
use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

#[derive(Clone, Default)]
pub struct CommandsApi {
    rpc_client: Option<Arc<RpcClient>>,
}

impl CommandsApi {
    pub fn new() -> Self {
        Self { rpc_client: None }
    }

    pub async fn init_rpc(&mut self, envd_url: &str, access_token: Option<&str>) -> Result<()> {
        let rpc_client = RpcClient::connect(envd_url, access_token).await?;
        self.rpc_client = Some(Arc::new(rpc_client));
        Ok(())
    }

    fn get_rpc_client(&self) -> Result<&Arc<RpcClient>> {
        self.rpc_client.as_ref().ok_or_else(|| Error::Api {
            status: 500,
            message: "RPC client not initialized. Call init_rpc first.".to_string(),
        })
    }

    pub async fn run(&self, cmd: &str) -> Result<CommandResult> {
        self.run_with_options(cmd, &CommandOptions::default()).await
    }

    pub async fn run_with_timeout(
        &self,
        cmd: &str,
        timeout_duration: Duration,
    ) -> Result<CommandResult> {
        let options = CommandOptions {
            timeout: Some(timeout_duration),
            ..Default::default()
        };
        self.run_with_options(cmd, &options).await
    }

    pub async fn run_background(&self, cmd: &str) -> Result<CommandHandle> {
        let options = CommandOptions {
            background: true,
            ..Default::default()
        };
        self.run_background_with_options(cmd, &options).await
    }

    pub async fn run_with_options(
        &self,
        cmd: &str,
        options: &CommandOptions,
    ) -> Result<CommandResult> {
        if options.background {
            return Err(Error::Api {
                status: 400,
                message: "Use run_background for background commands".to_string(),
            });
        }

        if let Some(timeout_duration) = options.timeout {
            timeout(timeout_duration, self.execute_command(cmd, options))
                .await
                .map_err(|_| Error::Timeout)?
        } else {
            self.execute_command(cmd, options).await
        }
    }

    pub async fn run_background_with_options(
        &self,
        cmd: &str,
        options: &CommandOptions,
    ) -> Result<CommandHandle> {
        self.start_command(cmd, options).await
    }

    async fn execute_command(&self, cmd: &str, options: &CommandOptions) -> Result<CommandResult> {
        let rpc_client = self.get_rpc_client()?;

        let (command, args) = Self::build_shell_command(cmd);

        // StartRequest has a ProcessConfig field named "process"
        let params = json!({
            "process": {
                "cmd": command,
                "args": args,
                "envs": options.envs.clone().unwrap_or_default(),
                "cwd": options.cwd
            }
        });

        let mut stream = rpc_client.process_start(params).await?;
        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut exit_code = None;
        let mut _pid = None;

        // Process all events from the stream
        while let Some(event) = stream.next_event().await? {
            match event.event {
                crate::rpc::ProcessEventData::Start { start } => {
                    _pid = Some(start.pid);
                }
                crate::rpc::ProcessEventData::Data { data } => {
                    if let Some(stdout_data) = &data.stdout {
                        // Decode Base64 stdout data
                        let decoded =
                            general_purpose::STANDARD.decode(stdout_data).map_err(|e| {
                                Error::Api {
                                    status: 500,
                                    message: format!("Failed to decode stdout: {}", e),
                                }
                            })?;
                        let text = String::from_utf8(decoded).map_err(|e| Error::Api {
                            status: 500,
                            message: format!("Failed to convert stdout to UTF-8: {}", e),
                        })?;
                        stdout.push_str(&text);
                    }
                    if let Some(stderr_data) = &data.stderr {
                        // Decode Base64 stderr data
                        let decoded =
                            general_purpose::STANDARD.decode(stderr_data).map_err(|e| {
                                Error::Api {
                                    status: 500,
                                    message: format!("Failed to decode stderr: {}", e),
                                }
                            })?;
                        let text = String::from_utf8(decoded).map_err(|e| Error::Api {
                            status: 500,
                            message: format!("Failed to convert stderr to UTF-8: {}", e),
                        })?;
                        stderr.push_str(&text);
                    }
                }
                crate::rpc::ProcessEventData::End { end } => {
                    if end.exited {
                        // Parse exit code from status string if available
                        if let Some(code) = end.exit_code {
                            exit_code = Some(code);
                        } else if end.status.contains("exit status") {
                            // Try to parse from "exit status X"
                            if let Some(code_str) = end.status.split("exit status ").nth(1) {
                                exit_code = code_str.trim().parse().ok();
                            }
                        }
                        break;
                    }
                }
            }
        }

        Ok(CommandResult {
            stdout,
            stderr,
            exit_code: exit_code.unwrap_or(-1),
            execution_time: None,
        })
    }

    async fn start_command(&self, cmd: &str, options: &CommandOptions) -> Result<CommandHandle> {
        let rpc_client = self.get_rpc_client()?;

        let (command, args) = Self::build_shell_command(cmd);

        // StartRequest has a ProcessConfig field named "process"
        let params = json!({
            "process": {
                "cmd": command,
                "args": args,
                "envs": options.envs.clone().unwrap_or_default(),
                "cwd": options.cwd
            }
        });

        let mut stream = rpc_client.process_start(params).await?;

        // Process all events in the stream to find the start event
        while let Some(event) = stream.next_event().await? {
            match event.event {
                crate::rpc::ProcessEventData::Start { start } => {
                    let pid = start.pid;

                    let (stdout_tx, stdout_rx) = mpsc::channel(100);
                    let (stderr_tx, stderr_rx) = mpsc::channel(100);
                    let (result_tx, result_rx) = oneshot::channel();

                    let mut stream = stream;
                    tokio::spawn(async move {
                        let stdout_sender = stdout_tx;
                        let stderr_sender = stderr_tx;
                        let mut stdout_acc = String::new();
                        let mut stderr_acc = String::new();
                        let mut exit_code = None;
                        let mut execution_time = None;

                        while let Ok(Some(event)) = stream.next_event().await {
                            match event.event {
                                crate::rpc::ProcessEventData::Data { data } => {
                                    if let Some(stdout_data) = data.stdout.as_ref() {
                                        if let Ok(decoded) =
                                            general_purpose::STANDARD.decode(stdout_data)
                                        {
                                            if let Ok(text) = String::from_utf8(decoded.clone()) {
                                                stdout_acc.push_str(&text);
                                                let _ = stdout_sender
                                                    .send(CommandOutput {
                                                        data: text,
                                                        timestamp: Utc::now(),
                                                    })
                                                    .await;
                                            }
                                        }
                                    }
                                    if let Some(stderr_data) = data.stderr.as_ref() {
                                        if let Ok(decoded) =
                                            general_purpose::STANDARD.decode(stderr_data)
                                        {
                                            if let Ok(text) = String::from_utf8(decoded.clone()) {
                                                stderr_acc.push_str(&text);
                                                let _ = stderr_sender
                                                    .send(CommandOutput {
                                                        data: text,
                                                        timestamp: Utc::now(),
                                                    })
                                                    .await;
                                            }
                                        }
                                    }
                                }
                                crate::rpc::ProcessEventData::End { end } => {
                                    if end.exited {
                                        exit_code = end.exit_code.or_else(|| {
                                            if end.status.contains("exit status") {
                                                end.status
                                                    .split("exit status ")
                                                    .nth(1)
                                                    .and_then(|s| s.trim().parse().ok())
                                            } else {
                                                None
                                            }
                                        });
                                    }
                                    execution_time = None;
                                    break;
                                }
                                crate::rpc::ProcessEventData::Start { .. } => {}
                            }
                        }

                        let _ = result_tx.send(CommandResult {
                            stdout: stdout_acc,
                            stderr: stderr_acc,
                            exit_code: exit_code.unwrap_or(-1),
                            execution_time,
                        });
                    });

                    return Ok(CommandHandle::new(pid, stdout_rx, stderr_rx, result_rx));
                }
                crate::rpc::ProcessEventData::Data { .. } => continue,
                crate::rpc::ProcessEventData::End { .. } => {
                    return Err(Error::Api {
                        status: 500,
                        message: "Process ended immediately after start".to_string(),
                    });
                }
            }
        }

        Err(Error::Api {
            status: 500,
            message: "Failed to start process: no PID received".to_string(),
        })
    }

    pub async fn wait_for_command(&self, handle: CommandHandle) -> Result<CommandResult> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({
            "process": {
                "pid": handle.pid()
            }
        });

        let mut stream = rpc_client.process_connect(params).await?;
        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut exit_code = None;

        // Read all events from the stream until process ends
        while let Some(event) = stream.next_event().await? {
            match event.event {
                crate::rpc::ProcessEventData::Data { data } => {
                    if let Some(stdout_data) = &data.stdout {
                        // Decode Base64 stdout data
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(stdout_data)
                            .map_err(|e| Error::Api {
                                status: 500,
                                message: format!("Failed to decode stdout: {}", e),
                            })?;
                        let text = String::from_utf8(decoded).map_err(|e| Error::Api {
                            status: 500,
                            message: format!("Failed to convert stdout to UTF-8: {}", e),
                        })?;
                        stdout.push_str(&text);
                    }
                    if let Some(stderr_data) = &data.stderr {
                        // Decode Base64 stderr data
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(stderr_data)
                            .map_err(|e| Error::Api {
                                status: 500,
                                message: format!("Failed to decode stderr: {}", e),
                            })?;
                        let text = String::from_utf8(decoded).map_err(|e| Error::Api {
                            status: 500,
                            message: format!("Failed to convert stderr to UTF-8: {}", e),
                        })?;
                        stderr.push_str(&text);
                    }
                }
                crate::rpc::ProcessEventData::End { end } => {
                    if end.exited {
                        // Parse exit code from status string if available
                        if let Some(code) = end.exit_code {
                            exit_code = Some(code);
                        } else if end.status.contains("exit status") {
                            // Try to parse from "exit status X"
                            if let Some(code_str) = end.status.split("exit status ").nth(1) {
                                exit_code = code_str.trim().parse().ok();
                            }
                        }
                        break;
                    }
                }
                crate::rpc::ProcessEventData::Start { .. } => {
                    // Skip start events in wait_for_command since we're already connected
                    continue;
                }
            }
        }

        Ok(CommandResult {
            stdout,
            stderr,
            exit_code: exit_code.unwrap_or(-1),
            execution_time: None,
        })
    }

    pub async fn list(&self) -> Result<Vec<ProcessInfo>> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({});
        let response = rpc_client.process_list(params).await?;

        // The response might be directly an array, have a "processes" field, or be empty
        let processes = if let Some(processes_array) = response.as_array() {
            // Response is directly an array
            processes_array
        } else if let Some(processes_array) = response["processes"].as_array() {
            // Response has a "processes" field
            processes_array
        } else if response.as_object().map_or(false, |obj| obj.is_empty()) {
            // Response is an empty object, meaning no processes
            return Ok(Vec::new());
        } else {
            return Err(Error::Api {
                status: 500,
                message: format!("Invalid response format: expected array or object with 'processes' field, got: {}", response),
            });
        };

        let mut result = Vec::new();
        for process in processes {
            let pid = process["pid"].as_u64().unwrap_or(0) as u32;
            let config = &process["config"];
            let cmd = config["cmd"].as_str().unwrap_or("").to_string();
            let args = config["args"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();
            let envs = config["envs"]
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let cwd = config["cwd"].as_str().map(|s| s.to_string());
            let tag = process["tag"].as_str().map(|s| s.to_string());

            result.push(ProcessInfo {
                pid,
                tag,
                cmd,
                args,
                envs,
                cwd,
            });
        }

        Ok(result)
    }

    pub async fn kill(&self, pid: u32) -> Result<bool> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({
            "process": {
                "pid": pid
            },
            "signal": "SIGNAL_SIGKILL"
        });

        match rpc_client.process_send_signal(params).await {
            Ok(_) => Ok(true),
            Err(Error::Api { status: 404, .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub async fn send_stdin(&self, pid: u32, data: &str) -> Result<()> {
        let rpc_client = self.get_rpc_client()?;

        // Encode stdin data as Base64
        let encoded_data = general_purpose::STANDARD.encode(data.as_bytes());

        let params = json!({
            "process": {
                "pid": pid
            },
            "input": {
                "stdin": encoded_data
            }
        });

        rpc_client.process_send_input(params).await?;
        Ok(())
    }

    pub async fn connect(&self, pid: u32) -> Result<CommandHandle> {
        // For HTTP-based implementation, connect just returns a handle to the existing process
        Ok(CommandHandle::from_pid(pid))
    }

    fn build_shell_command(cmd: &str) -> (String, Vec<String>) {
        (
            "/bin/bash".to_string(),
            vec!["-l".to_string(), "-c".to_string(), cmd.to_string()],
        )
    }
}
