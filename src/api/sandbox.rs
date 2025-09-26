use crate::{
    api::{CodeInterpreterApi, CommandsApi, FilesystemApi},
    client::Client,
    error::{Error, Result},
    models::{
        CodeExecution, Execution, LogLevel, Sandbox, SandboxCreateRequest, SandboxLog,
        SandboxMetrics,
    },
};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Clone)]
pub struct SandboxApi {
    client: Client,
}

impl SandboxApi {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn template(self, template_id: impl Into<String>) -> SandboxBuilder {
        SandboxBuilder::new(self.client, template_id.into())
    }

    pub async fn list(&self) -> Result<Vec<Sandbox>> {
        let url = self.client.build_url("/sandboxes");
        let response = self.client.http().get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let sandboxes: Vec<Sandbox> = response.json().await?;
                Ok(sandboxes)
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn get(&self, sandbox_id: &str) -> Result<Sandbox> {
        let url = self.client.build_url(&format!("/sandboxes/{}", sandbox_id));
        let response = self.client.http().get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let sandbox: Sandbox = response.json().await?;
                Ok(sandbox)
            }
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Sandbox {}", sandbox_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    async fn create_sandbox(&self, request: SandboxCreateRequest) -> Result<Sandbox> {
        let url = self.client.build_url("/sandboxes");
        let response = self.client.http().post(&url).json(&request).send().await?;

        match response.status() {
            StatusCode::CREATED | StatusCode::OK => {
                let response_text = response.text().await?;
                tracing::debug!("Sandbox creation response: {}", response_text);

                let sandbox: Sandbox =
                    serde_json::from_str(&response_text).map_err(|e| Error::Api {
                        status: 500,
                        message: format!(
                            "Failed to parse sandbox response: {}. Response: {}",
                            e, response_text
                        ),
                    })?;
                Ok(sandbox)
            }
            StatusCode::UNAUTHORIZED => Err(Error::Authentication("Invalid API key".to_string())),
            StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimit),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }
}

pub struct SandboxBuilder {
    client: Client,
    request: SandboxCreateRequest,
}

impl SandboxBuilder {
    fn new(client: Client, template_id: String) -> Self {
        Self {
            client,
            request: SandboxCreateRequest {
                template_id,
                timeout: None,
                auto_pause: None,
                secure: None,
                allow_internet_access: None,
                metadata: None,
                env_vars: None,
            },
        }
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.request.metadata = Some(metadata);
        self
    }

    pub fn timeout(mut self, seconds: u32) -> Self {
        self.request.timeout = Some(seconds);
        self
    }

    pub fn auto_pause(mut self, auto_pause: bool) -> Self {
        self.request.auto_pause = Some(auto_pause);
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.request.secure = Some(secure);
        self
    }

    pub fn allow_internet_access(mut self, allow: bool) -> Self {
        self.request.allow_internet_access = Some(allow);
        self
    }

    pub fn env_vars(mut self, env_vars: HashMap<String, String>) -> Self {
        self.request.env_vars = Some(env_vars);
        self
    }

    pub fn env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let env_vars = self.request.env_vars.get_or_insert_with(HashMap::new);
        env_vars.insert(key.into(), value.into());
        self
    }

    pub async fn create(self) -> Result<SandboxInstance> {
        let api = SandboxApi::new(self.client.clone());
        let sandbox = api.create_sandbox(self.request).await?;

        // Wait for sandbox to be fully ready before connecting RPC
        tracing::debug!("Waiting for sandbox to be ready...");
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Initialize Commands and Filesystem APIs with HTTP Connect protocol
        const ENVD_PORT: u16 = 49_983;
        let sandbox_domain = sandbox
            .sandbox_domain
            .clone()
            .or_else(|| sandbox.domain.clone())
            .unwrap_or_else(|| self.client.config().sandbox_domain());

        let envd_host = format!(
            "{}-{}.{}",
            ENVD_PORT,
            sandbox.sandbox_id,
            sandbox_domain.as_str()
        );

        let envd_scheme = "https";

        let envd_url = format!("{}://{}", envd_scheme, envd_host);
        tracing::debug!("Connecting to envd at: {}", envd_url);
        let access_token = sandbox.envd_access_token.as_deref();
        tracing::info!(
            sandbox_id = %sandbox.sandbox_id,
            envd_url = %envd_url,
            domain = ?sandbox.domain,
            sandbox_domain = ?sandbox.sandbox_domain,
            has_access_token = access_token.is_some(),
            "Configured sandbox envd endpoint"
        );

        let mut commands = CommandsApi::new();
        let mut files = FilesystemApi::new();

        // Try to initialize RPC with retries
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY: Duration = Duration::from_secs(2);

        while retry_count < MAX_RETRIES {
            match commands.init_rpc(&envd_url, access_token).await {
                Ok(()) => {
                    tracing::debug!("Commands RPC connected successfully");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        tracing::warn!("Failed to connect Commands RPC after {} retries: {}. Commands API will not be available.", MAX_RETRIES, e);
                        // Don't fail sandbox creation, just make commands unavailable
                        break;
                    }
                    tracing::warn!(
                        "Commands RPC connection failed (attempt {}/{}): {}",
                        retry_count,
                        MAX_RETRIES,
                        e
                    );
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }

        // Initialize filesystem RPC with same URL
        retry_count = 0;
        while retry_count < MAX_RETRIES {
            match files.init_rpc(&envd_url, access_token).await {
                Ok(()) => {
                    tracing::debug!("Filesystem RPC connected successfully");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        tracing::warn!("Failed to connect Filesystem RPC after {} retries: {}. Filesystem API will not be available.", MAX_RETRIES, e);
                        // Don't fail sandbox creation, just make filesystem unavailable
                        break;
                    }
                    tracing::warn!(
                        "Filesystem RPC connection failed (attempt {}/{}): {}",
                        retry_count,
                        MAX_RETRIES,
                        e
                    );
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }

        // Initialize code interpreter if using the code-interpreter template
        tracing::debug!(
            "Template ID: {}, Template Alias: {:?}",
            sandbox.template_id,
            sandbox.alias
        );
        let is_code_interpreter = sandbox.template_id.contains("code-interpreter")
            || sandbox
                .alias
                .as_ref()
                .map_or(false, |alias| alias.contains("code-interpreter"));

        let code_interpreter = if is_code_interpreter {
            tracing::debug!(
                "Initializing code interpreter for template: {} (alias: {:?})",
                sandbox.template_id,
                sandbox.alias
            );
            const JUPYTER_PORT: u16 = 49_999;
            let jupyter_host = format!(
                "{}-{}.{}",
                JUPYTER_PORT,
                sandbox.sandbox_id,
                sandbox_domain.as_str()
            );
            let jupyter_url = format!("{}://{}", envd_scheme, jupyter_host);
            let mut api = CodeInterpreterApi::new(self.client.clone(), jupyter_url.clone());
            if let Some(token) = access_token {
                api.set_envd_access_token(token.to_string());
            }
            tracing::info!(
                sandbox_id = %sandbox.sandbox_id,
                jupyter_url = %jupyter_url,
                "Configured code interpreter endpoint"
            );
            Some(api)
        } else {
            tracing::debug!("Code interpreter not initialized - neither template_id nor alias contains 'code-interpreter'");
            None
        };

        Ok(SandboxInstance {
            api,
            sandbox,
            commands,
            files,
            code_interpreter,
        })
    }
}

pub struct SandboxInstance {
    api: SandboxApi,
    sandbox: Sandbox,
    commands: CommandsApi,
    files: FilesystemApi,
    code_interpreter: Option<CodeInterpreterApi>,
}

impl SandboxInstance {
    pub fn id(&self) -> &str {
        &self.sandbox.sandbox_id
    }

    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    pub fn commands(&self) -> &CommandsApi {
        &self.commands
    }

    pub fn files(&self) -> &FilesystemApi {
        &self.files
    }

    pub fn code_interpreter(&self) -> Option<&CodeInterpreterApi> {
        self.code_interpreter.as_ref()
    }

    pub async fn run_code(&self, code: &str) -> Result<CodeExecution> {
        self.run_code_with_timeout(code, Duration::from_secs(30))
            .await
    }

    pub async fn run_code_with_language(&self, code: &str, language: &str) -> Result<Execution> {
        if let Some(interpreter) = &self.code_interpreter {
            // Add a small delay to ensure Jupyter server is ready
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            interpreter.run_code_with_language(code, language).await
        } else {
            Err(Error::Api {
                status: 400,
                message: format!("Code interpreter not available. Template ID: '{}', Alias: '{:?}'. Use 'code-interpreter-v1' template to enable code execution with language support.",
                    self.sandbox.template_id, self.sandbox.alias),
            })
        }
    }

    pub async fn run_python(&self, code: &str) -> Result<Execution> {
        self.run_code_with_language(code, "python").await
    }

    pub async fn run_javascript(&self, code: &str) -> Result<Execution> {
        self.run_code_with_language(code, "javascript").await
    }

    pub async fn run_code_with_timeout(
        &self,
        code: &str,
        timeout_duration: Duration,
    ) -> Result<CodeExecution> {
        let url = self
            .api
            .client
            .build_url(&format!("/sandboxes/{}/code", self.sandbox.sandbox_id));

        let request_body = serde_json::json!({
            "code": code
        });

        let request_future = async {
            let response = self
                .api
                .client
                .http()
                .post(&url)
                .json(&request_body)
                .send()
                .await?;

            match response.status() {
                StatusCode::OK => {
                    let execution: CodeExecution = response.json().await?;
                    Ok(execution)
                }
                StatusCode::NOT_FOUND => Err(Error::NotFound(format!(
                    "Sandbox {}",
                    self.sandbox.sandbox_id
                ))),
                status => {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(Error::Api {
                        status: status.as_u16(),
                        message: error_text,
                    })
                }
            }
        };

        timeout(timeout_duration, request_future)
            .await
            .map_err(|_| Error::Timeout)?
    }

    pub async fn pause(&self) -> Result<()> {
        let url = self
            .api
            .client
            .build_url(&format!("/sandboxes/{}/pause", self.sandbox.sandbox_id));
        let response = self
            .api
            .client
            .http()
            .post(&url)
            .json(&json!({}))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::debug!("pause response status={} body={}", status, body);

        match status {
            StatusCode::OK | StatusCode::NO_CONTENT | StatusCode::CREATED => Ok(()),
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!(
                "Sandbox {}",
                self.sandbox.sandbox_id
            ))),
            _ => Err(Error::Api {
                status: status.as_u16(),
                message: body,
            }),
        }
    }

    pub async fn resume(&self) -> Result<()> {
        let url = self
            .api
            .client
            .build_url(&format!("/sandboxes/{}/resume", self.sandbox.sandbox_id));
        let response = self
            .api
            .client
            .http()
            .post(&url)
            .json(&json!({}))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::debug!("resume response status={} body={}", status, body);

        match status {
            StatusCode::OK | StatusCode::NO_CONTENT | StatusCode::CREATED => Ok(()),
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!(
                "Sandbox {}",
                self.sandbox.sandbox_id
            ))),
            _ => Err(Error::Api {
                status: status.as_u16(),
                message: body,
            }),
        }
    }

    pub async fn delete(self) -> Result<()> {
        let url = self
            .api
            .client
            .build_url(&format!("/sandboxes/{}", self.sandbox.sandbox_id));
        let response = self.api.client.http().delete(&url).send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!(
                "Sandbox {}",
                self.sandbox.sandbox_id
            ))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn logs(&self) -> Result<Vec<SandboxLog>> {
        let url = self
            .api
            .client
            .build_url(&format!("/sandboxes/{}/logs", self.sandbox.sandbox_id));
        let response = self.api.client.http().get(&url).send().await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::debug!("sandbox logs response: {}", body);

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let value: Value = serde_json::from_str(&body).map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse logs response: {}", e),
        })?;

        let mut entries = Vec::new();

        if let Some(arr) = value.get("logEntries").and_then(|v| v.as_array()) {
            for item in arr {
                if let Ok(log) = Self::parse_structured_log(item) {
                    entries.push(log);
                }
            }
        }

        if let Some(arr) = value.get("logs").and_then(|v| v.as_array()) {
            for item in arr {
                if let Ok(log) = Self::parse_line_log(item) {
                    entries.push(log);
                }
            }
        }

        if entries.is_empty() {
            return Err(Error::Api {
                status: 500,
                message: "No log entries returned".to_string(),
            });
        }

        Ok(entries)
    }

    pub async fn metrics(&self) -> Result<SandboxMetrics> {
        let url = self
            .api
            .client
            .build_url(&format!("/sandboxes/{}/metrics", self.sandbox.sandbox_id));
        let response = self.api.client.http().get(&url).send().await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::debug!("sandbox metrics response: {}", body);

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let value: Value = serde_json::from_str(&body).map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse metrics response: {}", e),
        })?;

        if let Some(array) = value.as_array() {
            if let Some(first) = array.first() {
                return Self::parse_metrics(first);
            }
            return Ok(SandboxMetrics::default());
        }

        Self::parse_metrics(&value)
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.sandbox = self.api.get(&self.sandbox.sandbox_id).await?;
        Ok(())
    }

    fn parse_metrics(value: &Value) -> Result<SandboxMetrics> {
        let obj = value.as_object().ok_or_else(|| Error::Api {
            status: 500,
            message: "Invalid metrics format".to_string(),
        })?;

        Ok(SandboxMetrics {
            cpu_count: obj.get("cpuCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            cpu_used_pct: obj
                .get("cpuUsedPct")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            disk_total: obj.get("diskTotal").and_then(|v| v.as_u64()).unwrap_or(0),
            disk_used: obj.get("diskUsed").and_then(|v| v.as_u64()).unwrap_or(0),
            mem_total: obj.get("memTotal").and_then(|v| v.as_u64()).unwrap_or(0),
            mem_used: obj.get("memUsed").and_then(|v| v.as_u64()).unwrap_or(0),
            timestamp: Self::parse_timestamp(obj.get("timestamp")),
        })
    }

    fn parse_structured_log(value: &Value) -> Result<SandboxLog> {
        let obj = value.as_object().ok_or_else(|| Error::Api {
            status: 500,
            message: "Invalid log entry format".to_string(),
        })?;

        let level = obj.get("level").and_then(|v| v.as_str()).unwrap_or("info");

        let message = obj
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let timestamp = Self::parse_timestamp(obj.get("timestamp"));

        let fields = obj.get("fields").and_then(|v| v.as_object());
        let source = fields
            .and_then(|m| m.get("service").and_then(|v| v.as_str()))
            .or_else(|| fields.and_then(|m| m.get("logger").and_then(|v| v.as_str())))
            .unwrap_or("unknown")
            .to_string();

        Ok(SandboxLog {
            timestamp,
            level: Self::parse_log_level(level),
            message,
            source,
        })
    }

    fn parse_line_log(value: &Value) -> Result<SandboxLog> {
        let line = value.get("line").and_then(|v| v.as_str()).unwrap_or("");
        let timestamp = Self::parse_timestamp(value.get("timestamp"));

        if let Ok(parsed) = serde_json::from_str::<Value>(line) {
            if let Ok(mut log) = Self::parse_structured_log(&parsed) {
                log.timestamp = timestamp;
                return Ok(log);
            }
        }

        Ok(SandboxLog {
            timestamp,
            level: LogLevel::Info,
            message: line.to_string(),
            source: "log".to_string(),
        })
    }

    fn parse_log_level(level: &str) -> LogLevel {
        match level.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" | "warning" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }

    fn parse_timestamp(value: Option<&Value>) -> DateTime<Utc> {
        if let Some(Value::String(s)) = value {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return dt.with_timezone(&Utc);
            }
        }
        Utc::now()
    }
}
