use crate::{
    api::{CommandsApi, FilesystemApi, CodeInterpreterApi},
    client::Client,
    error::{Error, Result},
    models::{
        CodeExecution, Sandbox, SandboxCreateRequest, SandboxLog, SandboxMetrics, Execution,
    },
};
use reqwest::StatusCode;
use serde_json::Value;
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
        let response = self
            .client
            .http()
            .post(&url)
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::CREATED | StatusCode::OK => {
                let response_text = response.text().await?;
                tracing::debug!("Sandbox creation response: {}", response_text);

                let sandbox: Sandbox = serde_json::from_str(&response_text)
                    .map_err(|e| Error::Api {
                        status: 500,
                        message: format!("Failed to parse sandbox response: {}. Response: {}", e, response_text),
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
        // Based on Python SDK: envd_port = 49983, uses HTTP Connect not WebSocket
        let envd_url = format!("https://49983-{}.e2b.dev", sandbox.sandbox_id);
        tracing::debug!("Connecting to envd at: {}", envd_url);

        let mut commands = CommandsApi::new(self.client.clone(), sandbox.sandbox_id.clone());
        let mut files = FilesystemApi::new(self.client.clone(), sandbox.sandbox_id.clone());

        // Try to initialize RPC with retries
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY: Duration = Duration::from_secs(2);

        while retry_count < MAX_RETRIES {
            match commands.init_rpc(&envd_url).await {
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
                    tracing::warn!("Commands RPC connection failed (attempt {}/{}): {}", retry_count, MAX_RETRIES, e);
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }

        // Initialize filesystem RPC with same URL
        retry_count = 0;
        while retry_count < MAX_RETRIES {
            match files.init_rpc(&envd_url).await {
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
                    tracing::warn!("Filesystem RPC connection failed (attempt {}/{}): {}", retry_count, MAX_RETRIES, e);
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }

        // Initialize code interpreter if using the code-interpreter template
        tracing::debug!("Template ID: {}, Template Alias: {:?}", sandbox.template_id, sandbox.alias);
        let is_code_interpreter = sandbox.template_id.contains("code-interpreter") ||
            sandbox.alias.as_ref().map_or(false, |alias| alias.contains("code-interpreter"));

        let code_interpreter = if is_code_interpreter {
            tracing::debug!("Initializing code interpreter for template: {} (alias: {:?})", sandbox.template_id, sandbox.alias);
            let jupyter_url = format!("https://49999-{}.e2b.dev", sandbox.sandbox_id);
            Some(CodeInterpreterApi::new(self.client.clone(), sandbox.sandbox_id.clone(), jupyter_url))
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
        self.run_code_with_timeout(code, Duration::from_secs(30)).await
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

    pub async fn run_code_with_timeout(&self, code: &str, timeout_duration: Duration) -> Result<CodeExecution> {
        let url = self.api.client.build_url(&format!("/sandboxes/{}/code", self.sandbox.sandbox_id));

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
                StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Sandbox {}", self.sandbox.sandbox_id))),
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
        let url = self.api.client.build_url(&format!("/sandboxes/{}/pause", self.sandbox.sandbox_id));
        let response = self.api.client.http().post(&url).send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Sandbox {}", self.sandbox.sandbox_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn resume(&self) -> Result<()> {
        let url = self.api.client.build_url(&format!("/sandboxes/{}/resume", self.sandbox.sandbox_id));
        let response = self.api.client.http().post(&url).send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Sandbox {}", self.sandbox.sandbox_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn delete(self) -> Result<()> {
        let url = self.api.client.build_url(&format!("/sandboxes/{}", self.sandbox.sandbox_id));
        let response = self.api.client.http().delete(&url).send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Sandbox {}", self.sandbox.sandbox_id))),
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
        let url = self.api.client.build_url(&format!("/sandboxes/{}/logs", self.sandbox.sandbox_id));
        let response = self.api.client.http().get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let logs: Vec<SandboxLog> = response.json().await?;
                Ok(logs)
            }
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Sandbox {}", self.sandbox.sandbox_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn metrics(&self) -> Result<SandboxMetrics> {
        let url = self.api.client.build_url(&format!("/sandboxes/{}/metrics", self.sandbox.sandbox_id));
        let response = self.api.client.http().get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let metrics: SandboxMetrics = response.json().await?;
                Ok(metrics)
            }
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Sandbox {}", self.sandbox.sandbox_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.sandbox = self.api.get(&self.sandbox.sandbox_id).await?;
        Ok(())
    }
}