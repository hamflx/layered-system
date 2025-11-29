use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Normal,
    MissingFile,
    MissingParent,
    MissingBcd,
    Mounted,
    Error,
}

impl Default for NodeStatus {
    fn default() -> Self {
        NodeStatus::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub path: String,
    pub bcd_guid: Option<String>,
    pub desc: Option<String>,
    pub created_at: DateTime<Utc>,
    pub status: NodeStatus,
    pub boot_files_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WimImageInfo {
    pub index: u32,
    pub name: String,
    pub description: Option<String>,
    pub size: Option<String>,
}
