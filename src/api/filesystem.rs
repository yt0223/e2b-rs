use crate::{
    client::Client,
    error::{Error, Result},
    models::{
        EntryInfo, FileInfo, ReadFormat, ReadResult, WatchHandle, WriteEntry, WriteInfo
    },
    rpc::RpcClient,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct FilesystemApi {
    client: Client,
    rpc_client: Option<Arc<RpcClient>>,
    sandbox_id: String,
}

impl FilesystemApi {
    pub fn new(client: Client, sandbox_id: String) -> Self {
        Self {
            client,
            rpc_client: None,
            sandbox_id,
        }
    }

    pub async fn init_rpc(&mut self, envd_url: &str) -> Result<()> {
        let rpc_client = RpcClient::connect(envd_url).await?;
        self.rpc_client = Some(Arc::new(rpc_client));
        Ok(())
    }

    fn get_rpc_client(&self) -> Result<&Arc<RpcClient>> {
        self.rpc_client.as_ref().ok_or_else(|| Error::Api {
            status: 500,
            message: "RPC client not initialized. Call init_rpc first.".to_string(),
        })
    }

    pub async fn read_text(&self, path: &str) -> Result<String> {
        match self.read(path, ReadFormat::Text).await? {
            ReadResult::Text(content) => Ok(content),
            ReadResult::Binary(_) => Err(Error::Api {
                status: 500,
                message: "Unexpected binary result".to_string(),
            }),
        }
    }

    pub async fn read_binary(&self, path: &str) -> Result<Vec<u8>> {
        match self.read(path, ReadFormat::Binary).await? {
            ReadResult::Binary(content) => Ok(content),
            ReadResult::Text(_) => Err(Error::Api {
                status: 500,
                message: "Unexpected text result".to_string(),
            }),
        }
    }

    pub async fn read(&self, path: &str, format: ReadFormat) -> Result<ReadResult> {
        let rpc_client = self.get_rpc_client()?;

        // Use the HTTP GET endpoint like the Python SDK
        let content = rpc_client.filesystem_read(path, "user").await?;

        match format {
            ReadFormat::Text => Ok(ReadResult::Text(content)),
            ReadFormat::Binary => {
                // If we need binary, decode from base64
                use base64::{Engine, engine::general_purpose};
                let decoded = general_purpose::STANDARD.decode(&content).map_err(|e| Error::Api {
                    status: 500,
                    message: format!("Failed to decode binary content: {}", e),
                })?;
                Ok(ReadResult::Binary(decoded))
            }
        }
    }

    pub async fn write_text(&self, path: &str, content: &str) -> Result<WriteInfo> {
        let entry = WriteEntry::text(path, content);
        self.write(entry).await
    }

    pub async fn write_binary(&self, path: &str, content: Vec<u8>) -> Result<WriteInfo> {
        let entry = WriteEntry::binary(path, content);
        self.write(entry).await
    }

    pub async fn write(&self, entry: WriteEntry) -> Result<WriteInfo> {
        let rpc_client = self.get_rpc_client()?;

        let (content, format) = match entry.data {
            crate::models::WriteData::Text(text) => (text, "text"),
            crate::models::WriteData::Binary(bytes) => {
                use base64::{Engine, engine::general_purpose};
                (general_purpose::STANDARD.encode(bytes), "binary")
            }
        };

        let params = json!({
            "path": entry.path,
            "content": content,
            "format": format,
            "username": "user"
        });

        let response = rpc_client.filesystem_write(params).await?;

        let path = response["path"].as_str()
            .ok_or_else(|| Error::Api {
                status: 500,
                message: "Invalid response: missing path".to_string(),
            })?;

        let size = response["size"].as_u64()
            .ok_or_else(|| Error::Api {
                status: 500,
                message: "Invalid response: missing size".to_string(),
            })?;

        Ok(WriteInfo {
            path: path.to_string(),
            size,
        })
    }

    pub async fn write_files(&self, entries: Vec<WriteEntry>) -> Result<Vec<WriteInfo>> {
        let rpc_client = self.get_rpc_client()?;

        let files: Vec<Value> = entries.into_iter().map(|entry| {
            let (content, format) = match entry.data {
                crate::models::WriteData::Text(text) => (text, "text"),
                crate::models::WriteData::Binary(bytes) => {
                    use base64::{Engine, engine::general_purpose};
                    (general_purpose::STANDARD.encode(bytes), "binary")
                }
            };

            json!({
                "path": entry.path,
                "content": content,
                "format": format
            })
        }).collect();

        let params = json!({
            "files": files,
            "username": "user"
        });

        let response = rpc_client.filesystem_write(params).await?;

        let results = response.as_array()
            .ok_or_else(|| Error::Api {
                status: 500,
                message: "Invalid response format".to_string(),
            })?;

        let mut write_infos = Vec::new();
        for result in results {
            let path = result["path"].as_str()
                .ok_or_else(|| Error::Api {
                    status: 500,
                    message: "Invalid response: missing path".to_string(),
                })?;

            let size = result["size"].as_u64()
                .ok_or_else(|| Error::Api {
                    status: 500,
                    message: "Invalid response: missing size".to_string(),
                })?;

            write_infos.push(WriteInfo {
                path: path.to_string(),
                size,
            });
        }

        Ok(write_infos)
    }

    pub async fn list(&self, path: &str) -> Result<Vec<EntryInfo>> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({
            "path": path,
            "username": "user"
        });

        let response = rpc_client.filesystem_list(params).await?;
        let entries = response["entries"].as_array()
            .ok_or_else(|| Error::Api {
                status: 500,
                message: "Invalid response format: missing entries".to_string(),
            })?;

        let mut result = Vec::new();
        for entry in entries {
            let path = entry["path"].as_str().unwrap_or("").to_string();
            let name = entry["name"].as_str().unwrap_or("").to_string();
            let is_dir = entry["is_dir"].as_bool().unwrap_or(false);
            let size = entry["size"].as_u64().unwrap_or(0);
            let created_at = entry["created_at"].as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            let updated_at = entry["updated_at"].as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            let permissions = entry["permissions"].as_str().unwrap_or("").to_string();

            result.push(EntryInfo {
                path,
                name,
                is_dir,
                size,
                created_at,
                updated_at,
                permissions,
            });
        }

        Ok(result)
    }

    pub async fn exists(&self, path: &str) -> Result<bool> {
        let params = json!({
            "path": path,
            "username": "user"
        });

        let rpc_client = self.get_rpc_client()?;
        match rpc_client.filesystem_stat(params).await {
            Ok(_) => Ok(true),
            Err(Error::Api { status: 404, .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub async fn get_info(&self, path: &str) -> Result<FileInfo> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({
            "path": path,
            "username": "user"
        });

        let response = rpc_client.filesystem_stat(params).await?;

        let path = response["path"].as_str().unwrap_or("").to_string();
        let name = response["name"].as_str().unwrap_or("").to_string();
        let size = response["size"].as_u64().unwrap_or(0);
        let is_dir = response["is_dir"].as_bool().unwrap_or(false);
        let created_at = response["created_at"].as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        let modified_at = response["modified_at"].as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        let permissions = response["permissions"].as_u64().unwrap_or(0) as u32;
        let owner = response["owner"].as_str().unwrap_or("").to_string();
        let group = response["group"].as_str().unwrap_or("").to_string();

        Ok(FileInfo {
            path,
            name,
            size,
            is_dir,
            created_at,
            modified_at,
            permissions,
            owner,
            group,
        })
    }

    pub async fn remove(&self, path: &str) -> Result<()> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({
            "path": path,
            "username": "user"
        });

        rpc_client.filesystem_remove(params).await?;
        Ok(())
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({
            "from": from,
            "to": to,
            "username": "user"
        });

        rpc_client.filesystem_move(params).await?;
        Ok(())
    }

    pub async fn make_dir(&self, path: &str) -> Result<()> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({
            "path": path,
            "username": "user"
        });

        rpc_client.filesystem_make_dir(params).await?;
        Ok(())
    }

    pub async fn watch_dir(&self, path: &str) -> Result<WatchHandle> {
        // For now, return a simple watch handle
        // In a full implementation, this would set up streaming of filesystem events
        let (handle, _event_sender, _stop_receiver) = WatchHandle::new(path.to_string());
        Ok(handle)
    }
}