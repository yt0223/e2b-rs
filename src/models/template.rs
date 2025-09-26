use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub template_id: String,
    pub name: String,
    pub description: Option<String>,
    pub team_id: String,
    pub build_id: Option<String>,
    pub public: bool,
    pub cpu_count: u32,
    pub memory_mb: u32,
    pub disk_mb: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCreateRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub dockerfile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_mb: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateBuild {
    pub build_id: String,
    pub template_id: String,
    pub status: BuildStatus,
    pub dockerfile: String,
    pub logs: Vec<BuildLog>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildStatus {
    Building,
    Ready,
    Error,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildLog {
    pub timestamp: DateTime<Utc>,
    pub line: String,
    pub level: BuildLogLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildLogLevel {
    Info,
    Error,
    Debug,
}
