use crate::{
    error::{Error, Result},
    models::{EntryInfo, FileInfo, ReadFormat, ReadResult, WatchHandle, WriteEntry, WriteInfo},
    rpc::RpcClient,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct FilesystemApi {
    rpc_client: Option<Arc<RpcClient>>,
}

impl FilesystemApi {
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
            ReadFormat::Binary => Ok(ReadResult::Binary(content.into_bytes())),
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
        let entries = vec![entry];
        let mut results = self.upload_files(&entries).await?;
        results.pop().ok_or_else(|| Error::Api {
            status: 500,
            message: "Write operation returned no result".to_string(),
        })
    }

    pub async fn write_files(&self, entries: Vec<WriteEntry>) -> Result<Vec<WriteInfo>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        self.upload_files(&entries).await
    }

    async fn upload_files(&self, entries: &[WriteEntry]) -> Result<Vec<WriteInfo>> {
        let rpc_client = self.get_rpc_client()?;
        rpc_client.filesystem_upload(entries, "user").await
    }

    fn parse_entry_info(value: &Value) -> Result<EntryInfo> {
        let entry = value.as_object().ok_or_else(|| Error::Api {
            status: 500,
            message: "Invalid entry format".to_string(),
        })?;

        let path = entry
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let entry_type = entry
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("FILE_TYPE_FILE");
        let is_dir = entry_type == "FILE_TYPE_DIRECTORY";
        let size = Self::parse_size(entry.get("size"));
        let modified_at = Self::parse_timestamp(entry.get("modifiedTime"));
        let permissions = entry
            .get("permissions")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(EntryInfo {
            path,
            name,
            is_dir,
            size,
            created_at: modified_at,
            updated_at: modified_at,
            permissions,
        })
    }

    fn parse_file_info(entry: &serde_json::Map<String, Value>) -> Result<FileInfo> {
        let path = entry
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let entry_type = entry
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("FILE_TYPE_FILE");
        let is_dir = entry_type == "FILE_TYPE_DIRECTORY";
        let size = Self::parse_size(entry.get("size"));
        let modified_at = Self::parse_timestamp(entry.get("modifiedTime"));
        let created_at = entry
            .get("createdTime")
            .map(|v| Self::parse_timestamp(Some(v)))
            .unwrap_or(modified_at);
        let permissions = entry.get("mode").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
        let owner = entry
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let group = entry
            .get("group")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

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

    fn parse_size(value: Option<&Value>) -> u64 {
        if let Some(v) = value {
            if let Some(n) = v.as_u64() {
                return n;
            }
            if let Some(s) = v.as_str() {
                return s.parse().unwrap_or(0);
            }
            if let Some(n) = v.as_i64() {
                return n.max(0) as u64;
            }
        }
        0
    }

    fn parse_timestamp(value: Option<&Value>) -> DateTime<Utc> {
        if let Some(Value::String(s)) = value {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return dt.with_timezone(&Utc);
            }
        }
        Utc::now()
    }

    pub async fn list(&self, path: &str) -> Result<Vec<EntryInfo>> {
        let rpc_client = self.get_rpc_client()?;

        let params = json!({
            "path": path,
            "username": "user"
        });

        let response = rpc_client.filesystem_list(params).await?;
        tracing::debug!("filesystem list response: {}", response);
        let entries = response["entries"].as_array().ok_or_else(|| Error::Api {
            status: 500,
            message: "Invalid response format: missing entries".to_string(),
        })?;

        entries.iter().map(Self::parse_entry_info).collect()
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
        tracing::debug!("filesystem stat response: {}", response);

        let entry = response["entry"].as_object().ok_or_else(|| Error::Api {
            status: 500,
            message: "Invalid response format: missing entry".to_string(),
        })?;

        Self::parse_file_info(entry)
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
            "source": from,
            "destination": to,
            "username": "user"
        });

        let response = rpc_client.filesystem_move(params).await?;
        tracing::debug!("filesystem move response: {}", response);
        let entry = response["entry"].clone();
        Self::parse_entry_info(&entry)?;
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
