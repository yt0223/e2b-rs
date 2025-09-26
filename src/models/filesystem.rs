use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryInfo {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub permissions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteInfo {
    pub path: String,
    pub name: String,
    #[serde(rename = "type", default)]
    pub entry_type: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct WriteEntry {
    pub path: String,
    pub data: WriteData,
}

#[derive(Debug, Clone)]
pub enum WriteData {
    Text(String),
    Binary(Vec<u8>),
}

impl WriteEntry {
    pub fn text(path: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            data: WriteData::Text(data.into()),
        }
    }

    pub fn binary(path: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            data: WriteData::Binary(data),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub permissions: u32,
    pub owner: String,
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilesystemEventType {
    Create,
    Modify,
    Delete,
    Move,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemEvent {
    pub event_type: FilesystemEventType,
    pub path: String,
    pub timestamp: DateTime<Utc>,
    pub old_path: Option<String>,
}

#[derive(Debug)]
pub struct WatchHandle {
    pub path: String,
    event_receiver: tokio::sync::mpsc::Receiver<FilesystemEvent>,
    stop_sender: tokio::sync::oneshot::Sender<()>,
}

impl WatchHandle {
    pub fn new(
        path: String,
    ) -> (
        Self,
        tokio::sync::mpsc::Sender<FilesystemEvent>,
        tokio::sync::oneshot::Receiver<()>,
    ) {
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(100);
        let (stop_sender, stop_receiver) = tokio::sync::oneshot::channel();

        let handle = Self {
            path,
            event_receiver,
            stop_sender,
        };

        (handle, event_sender, stop_receiver)
    }

    pub async fn stop(self) -> Result<(), crate::Error> {
        self.stop_sender.send(()).map_err(|_| crate::Error::Api {
            status: 500,
            message: "Failed to stop watch".to_string(),
        })?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Option<FilesystemEvent> {
        self.event_receiver.recv().await
    }
}

#[derive(Debug, Clone)]
pub enum ReadFormat {
    Text,
    Binary,
}

#[derive(Debug, Clone)]
pub enum ReadResult {
    Text(String),
    Binary(Vec<u8>),
}
