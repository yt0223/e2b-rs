use crate::{
    models::{WriteData, WriteEntry, WriteInfo},
    Error, Result,
};
use base64::{engine::general_purpose, Engine};
use bytes::BytesMut;
use futures::{stream::BoxStream, StreamExt};
use http::HeaderMap;
use reqwest::{
    multipart::{Form, Part},
    Client as HttpClient, Response,
};
use serde_json::Value;
use std::collections::VecDeque;
use tracing::debug;

pub struct RpcClient {
    base_url: String,
    http_client: HttpClient,
    headers: HeaderMap,
}

impl RpcClient {
    pub async fn connect(url: impl Into<String>, access_token: Option<&str>) -> Result<Self> {
        let base_url = url.into();
        let http_client = HttpClient::new();
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("connect-protocol-version", "1".parse().unwrap());
        headers.insert("content-encoding", "identity".parse().unwrap());

        // Add Basic Auth header for user authentication
        // Using "user:" (username:password, but password is empty)
        let auth_value = general_purpose::STANDARD.encode("user:");
        headers.insert(
            "Authorization",
            format!("Basic {}", auth_value).parse().unwrap(),
        );

        if let Some(token) = access_token {
            headers.insert(
                "X-Access-Token",
                token.parse().map_err(|e| Error::Api {
                    status: 400,
                    message: format!("Invalid access token header: {}", e),
                })?,
            );
        }

        Ok(Self {
            base_url,
            http_client,
            headers,
        })
    }

    pub fn set_header(&mut self, name: &'static str, value: &str) -> Result<()> {
        self.headers.insert(
            name,
            value.parse().map_err(|e| Error::Api {
                status: 400,
                message: format!("Invalid header value: {}", e),
            })?,
        );
        Ok(())
    }

    async fn post_connect_request(
        &self,
        service: &str,
        method: &str,
        request: Value,
        is_stream: bool,
    ) -> Result<Response> {
        let url = format!("{}/{}/{}", self.base_url, service, method);

        debug!("Making Connect request to: {}", url);
        debug!("Request body: {}", request);

        let mut headers = self.headers.clone();

        // Use different Content-Type based on whether it's a streaming request
        let content_type = if is_stream {
            "application/connect+json"
        } else {
            "application/json"
        };
        headers.insert("Content-Type", content_type.parse().unwrap());

        // For Connect protocol, we need to wrap the request in an envelope
        let json_data = serde_json::to_string(&request).map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to serialize request: {}", e),
        })?;

        let body = if is_stream {
            // For streaming requests, wrap in Connect envelope format
            create_connect_envelope(&json_data)
        } else {
            json_data.into_bytes()
        };

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Api {
                status: 500,
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Api {
                status,
                message: format!("HTTP {} error: {}", status, body),
            });
        }

        Ok(response)
    }

    // Process service calls using Connect protocol
    pub async fn process_list(&self, _params: Value) -> Result<Value> {
        // ListRequest is empty according to the protobuf
        let request = serde_json::json!({});
        let response = self
            .post_connect_request("process.Process", "List", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;

        debug!("Process list response: {}", result);
        Ok(result)
    }

    pub async fn process_start(&self, params: Value) -> Result<ProcessStream> {
        let request = params;
        let response = self
            .post_connect_request("process.Process", "Start", request, true)
            .await?;
        ProcessStream::new(response).await
    }

    pub async fn process_send_input(&self, params: Value) -> Result<Value> {
        let request = params;
        let response = self
            .post_connect_request("process.Process", "SendInput", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;
        Ok(result)
    }

    pub async fn process_send_signal(&self, params: Value) -> Result<Value> {
        let request = params;
        let response = self
            .post_connect_request("process.Process", "SendSignal", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;
        Ok(result)
    }

    pub async fn process_connect(&self, params: Value) -> Result<ProcessStream> {
        let request = params;
        let response = self
            .post_connect_request("process.Process", "Connect", request, true)
            .await?;
        ProcessStream::new(response).await
    }

    // Filesystem service calls using Connect protocol
    pub async fn filesystem_read(&self, path: &str, username: &str) -> Result<String> {
        // For filesystem read, we might need to use a different approach
        // Let's try the files endpoint first as that might be a REST endpoint
        let url = format!(
            "{}/files?path={}&username={}",
            self.base_url, path, username
        );

        let response = self
            .http_client
            .get(&url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| Error::Api {
                status: 500,
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Api {
                status,
                message: format!("HTTP {} error: {}", status, body),
            });
        }

        response.text().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to read response: {}", e),
        })
    }

    pub async fn filesystem_write(&self, params: Value) -> Result<Value> {
        let request = params;
        let response = self
            .post_connect_request("filesystem.Filesystem", "Write", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;
        Ok(result)
    }

    pub async fn filesystem_upload(
        &self,
        entries: &[WriteEntry],
        username: &str,
    ) -> Result<Vec<WriteInfo>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/files", self.base_url);
        let mut form = Form::new();

        for entry in entries {
            let part = match &entry.data {
                WriteData::Text(text) => Part::text(text.clone()),
                WriteData::Binary(bytes) => Part::bytes(bytes.clone()),
            }
            .file_name(entry.path.clone());

            form = form.part("file", part);
        }

        let mut headers = self.headers.clone();
        headers.remove("Content-Type");

        let mut request = self
            .http_client
            .post(&url)
            .headers(headers)
            .query(&[("username", username)]);

        if entries.len() == 1 {
            request = request.query(&[("path", entries[0].path.as_str())]);
        }

        let response = request
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::Api {
                status: 500,
                message: format!("HTTP request failed: {}", e),
            })?;

        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "".to_string());

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: format!(
                    "HTTP {} error: {}",
                    status.as_u16(),
                    if body.is_empty() {
                        "Unknown error"
                    } else {
                        &body
                    }
                )
                .to_string(),
            });
        }

        tracing::debug!("filesystem upload response body: {}", body);

        serde_json::from_str::<Vec<WriteInfo>>(&body).map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })
    }

    pub async fn filesystem_list(&self, params: Value) -> Result<Value> {
        let request = params;
        let response = self
            .post_connect_request("filesystem.Filesystem", "ListDir", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;
        Ok(result)
    }

    pub async fn filesystem_stat(&self, params: Value) -> Result<Value> {
        let request = params;
        let response = self
            .post_connect_request("filesystem.Filesystem", "Stat", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;
        Ok(result)
    }

    pub async fn filesystem_make_dir(&self, params: Value) -> Result<Value> {
        let request = params;
        let response = self
            .post_connect_request("filesystem.Filesystem", "MakeDir", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;
        Ok(result)
    }

    pub async fn filesystem_remove(&self, params: Value) -> Result<Value> {
        let request = params;
        let response = self
            .post_connect_request("filesystem.Filesystem", "Remove", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;
        Ok(result)
    }

    pub async fn filesystem_move(&self, params: Value) -> Result<Value> {
        let request = params;
        let response = self
            .post_connect_request("filesystem.Filesystem", "Move", request, false)
            .await?;
        let result: Value = response.json().await.map_err(|e| Error::Api {
            status: 500,
            message: format!("Failed to parse response: {}", e),
        })?;
        Ok(result)
    }
}

// Create Connect protocol envelope
fn create_connect_envelope(data: &str) -> Vec<u8> {
    let data_bytes = data.as_bytes();
    let mut envelope = Vec::new();

    // Connect envelope header: 1 byte flags + 4 bytes length (big-endian)
    envelope.push(0); // flags: no compression, not end stream
    envelope.extend_from_slice(&(data_bytes.len() as u32).to_be_bytes());
    envelope.extend_from_slice(data_bytes);

    envelope
}

// Streaming wrapper around Connect envelope responses
// Simple struct to handle streaming process output
pub struct ProcessStream {
    stream: BoxStream<'static, reqwest::Result<bytes::Bytes>>,
    buffer: BytesMut,
    messages: VecDeque<String>,
    finished: bool,
}

impl ProcessStream {
    pub async fn new(response: Response) -> Result<Self> {
        let stream = response.bytes_stream().boxed();

        Ok(Self {
            stream,
            buffer: BytesMut::new(),
            messages: VecDeque::new(),
            finished: false,
        })
    }

    pub async fn next_event(&mut self) -> Result<Option<ProcessEvent>> {
        loop {
            if let Some(message) = self.messages.pop_front() {
                let trimmed = message.trim();

                debug!("Processing message: {}", message);

                if trimmed.is_empty() || trimmed == "{}" {
                    if self.finished && self.messages.is_empty() {
                        return Ok(None);
                    }
                    continue;
                }

                if let Ok(error_resp) = serde_json::from_str::<serde_json::Value>(&message) {
                    if let Some(error) = error_resp.get("error") {
                        return Err(Error::Api {
                            status: 500,
                            message: format!(
                                "Server error: {}",
                                error
                                    .get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("Unknown error")
                            ),
                        });
                    }
                }

                let event: ProcessEvent =
                    serde_json::from_str(&message).map_err(|e| Error::Api {
                        status: 500,
                        message: format!("Failed to parse process event: {}", e),
                    })?;

                return Ok(Some(event));
            }

            if self.finished {
                return Ok(None);
            }

            match self.stream.next().await {
                Some(Ok(chunk)) => {
                    self.buffer.extend_from_slice(&chunk);
                    self.extract_messages()?;
                }
                Some(Err(e)) => {
                    return Err(Error::Api {
                        status: 500,
                        message: format!("Failed to read stream: {}", e),
                    });
                }
                None => {
                    self.finished = true;
                    // Consume any pending buffered messages before exiting
                    self.extract_messages()?;
                }
            }
        }
    }

    fn extract_messages(&mut self) -> Result<()> {
        loop {
            if self.buffer.len() < 5 {
                return Ok(());
            }

            let length = u32::from_be_bytes([
                self.buffer[1],
                self.buffer[2],
                self.buffer[3],
                self.buffer[4],
            ]) as usize;

            if self.buffer.len() < 5 + length {
                return Ok(());
            }

            let frame = self.buffer.split_to(5 + length);
            let flags = frame[0];
            let payload = &frame[5..];

            let message = String::from_utf8(payload.to_vec()).map_err(|e| Error::Api {
                status: 500,
                message: format!("Failed to decode message: {}", e),
            })?;

            if flags & 0b0000_0010 != 0 {
                self.finished = true;
            }

            self.messages.push_back(message);
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProcessEvent {
    pub event: ProcessEventData,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum ProcessEventData {
    Start { start: ProcessStart },
    Data { data: ProcessData },
    End { end: ProcessEnd },
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProcessStart {
    pub pid: u32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProcessData {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProcessEnd {
    pub exited: bool,
    pub status: String,
    pub exit_code: Option<i32>,
}
