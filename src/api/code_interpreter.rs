use crate::{
    client::Client,
    error::{Error, Result as ApiResult},
    models::{CodeExecutionRequest, CodeInterpreterOptions, Context, Execution},
};
use reqwest::StatusCode;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Clone)]
pub struct CodeInterpreterApi {
    client: Client,
    jupyter_url: String,
    envd_access_token: Option<String>,
}

impl CodeInterpreterApi {
    pub fn new(client: Client, jupyter_url: String) -> Self {
        Self {
            client,
            jupyter_url,
            envd_access_token: None,
        }
    }

    pub fn set_envd_access_token(&mut self, token: String) {
        self.envd_access_token = Some(token);
    }

    pub async fn run_code(&self, code: &str) -> ApiResult<Execution> {
        let options = CodeInterpreterOptions::default();
        self.run_code_with_options(code, &options).await
    }

    pub async fn run_code_with_language(&self, code: &str, language: &str) -> ApiResult<Execution> {
        let options = CodeInterpreterOptions {
            language: Some(language.to_string()),
            ..Default::default()
        };
        self.run_code_with_options(code, &options).await
    }

    pub async fn run_code_with_options(
        &self,
        code: &str,
        options: &CodeInterpreterOptions,
    ) -> ApiResult<Execution> {
        let request = CodeExecutionRequest {
            code: code.to_string(),
            language: options.language.clone(),
            context_id: options.context.as_ref().map(|c| c.id.clone()),
            env_vars: options.env_vars.clone(),
        };

        let timeout_duration = options.timeout.unwrap_or(Duration::from_secs(300));

        let request_future = async {
            let url = format!("{}/execute", self.jupyter_url);
            let mut request_builder = self.client.http().post(&url).json(&request);

            if let Some(token) = &self.envd_access_token {
                request_builder = request_builder.header("X-Access-Token", token);
            }

            let response = request_builder.send().await?;

            match response.status() {
                StatusCode::OK => {
                    let text = response.text().await?;
                    tracing::debug!("Jupyter response: {}", text);
                    self.parse_jupyter_response(&text).await
                }
                StatusCode::NOT_FOUND => Err(Error::NotFound(format!(
                    "Jupyter server not found at {}",
                    url
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

    async fn parse_jupyter_response(&self, response_text: &str) -> ApiResult<Execution> {
        // Parse streaming JSON lines from Jupyter response
        tracing::debug!("Parsing Jupyter response, {} chars", response_text.len());
        let mut execution = Execution {
            stdout: String::new(),
            stderr: String::new(),
            results: Vec::new(),
            error: None,
            is_main_result: false,
        };

        let lines: Vec<&str> = response_text.lines().collect();
        tracing::debug!("Response has {} lines", lines.len());

        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            tracing::debug!("Line {}: {}", i, line);
            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(json) => {
                    tracing::debug!(
                        "Parsed JSON keys: {:?}",
                        json.as_object().map(|o| o.keys().collect::<Vec<_>>())
                    );

                    // Check for different possible response formats
                    if let Some(msg_type) = json.get("type").and_then(|t| t.as_str()) {
                        tracing::debug!("Message type: {}", msg_type);
                        match msg_type {
                            "stdout" => {
                                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                                    execution.stdout.push_str(text);
                                } else if let Some(data) = json.get("line").and_then(|l| l.as_str())
                                {
                                    execution.stdout.push_str(data);
                                    execution.stdout.push('\n');
                                } else if let Some(data) = json.get("data").and_then(|l| l.as_str())
                                {
                                    execution.stdout.push_str(data);
                                    execution.stdout.push('\n');
                                }
                            }
                            "stderr" => {
                                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                                    execution.stderr.push_str(text);
                                } else if let Some(data) = json.get("line").and_then(|l| l.as_str())
                                {
                                    execution.stderr.push_str(data);
                                    execution.stderr.push('\n');
                                } else if let Some(data) = json.get("data").and_then(|l| l.as_str())
                                {
                                    execution.stderr.push_str(data);
                                    execution.stderr.push('\n');
                                }
                            }
                            "result" | "display_data" => {
                                let mut result_data = std::collections::HashMap::new();

                                // Check for text result
                                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                                    result_data.insert("text/plain".to_string(), text.to_string());
                                }

                                // Check for other data fields
                                if let Some(data) = json.get("data") {
                                    if let Some(data_obj) = data.as_object() {
                                        for (k, v) in data_obj {
                                            if let Some(v_str) = v.as_str() {
                                                result_data.insert(k.clone(), v_str.to_string());
                                            }
                                        }
                                    }
                                }

                                if !result_data.is_empty() {
                                    execution.results.push(
                                        crate::models::code_interpreter::Result {
                                            result_type: msg_type.to_string(),
                                            data: result_data,
                                        },
                                    );
                                    execution.is_main_result = json
                                        .get("is_main_result")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(true);
                                }
                            }
                            "error" => {
                                execution.error = Some(crate::models::ExecutionError {
                                    name: json
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("Unknown")
                                        .to_string(),
                                    value: json
                                        .get("value")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    traceback: json
                                        .get("traceback")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                });
                            }
                            _ => {
                                tracing::debug!("Unknown message type: {}", msg_type);
                            }
                        }
                    } else {
                        // Maybe the response has a different structure
                        tracing::debug!("No 'type' field found, checking for other patterns");

                        // Check if it's a direct output response
                        if let Some(stdout) = json.get("stdout").and_then(|s| s.as_str()) {
                            execution.stdout.push_str(stdout);
                        }
                        if let Some(stderr) = json.get("stderr").and_then(|s| s.as_str()) {
                            execution.stderr.push_str(stderr);
                        }
                    }
                }
                Err(_) => {
                    // Skip malformed JSON lines
                    continue;
                }
            }
        }

        tracing::debug!(
            "Final execution result - stdout: '{}', stderr: '{}', results: {}, error: {:?}",
            execution.stdout,
            execution.stderr,
            execution.results.len(),
            execution.error.is_some()
        );
        Ok(execution)
    }

    pub async fn create_context(
        &self,
        language: Option<&str>,
        cwd: Option<&str>,
    ) -> ApiResult<Context> {
        let mut request_data = HashMap::new();
        if let Some(lang) = language {
            request_data.insert("language", lang);
        }
        if let Some(working_dir) = cwd {
            request_data.insert("cwd", working_dir);
        }

        let url = format!("{}/contexts", self.jupyter_url);
        let mut request_builder = self.client.http().post(&url).json(&request_data);

        if let Some(token) = &self.envd_access_token {
            request_builder = request_builder.header("X-Access-Token", token);
        }

        let response = request_builder.send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let context: Context = response.json().await?;
                Ok(context)
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

    pub async fn list_contexts(&self) -> ApiResult<Vec<Context>> {
        let url = format!("{}/contexts", self.jupyter_url);
        let mut request_builder = self.client.http().get(&url);

        if let Some(token) = &self.envd_access_token {
            request_builder = request_builder.header("X-Access-Token", token);
        }

        let response = request_builder.send().await?;

        match response.status() {
            StatusCode::OK => {
                let contexts: Vec<Context> = response.json().await?;
                Ok(contexts)
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
}
