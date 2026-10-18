use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShareItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub read_only: bool,
    pub guest_access: bool,
    pub exists: bool,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Enabled,
    Disabled,
    Unknown,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: ServiceStatus,
    pub shares: Vec<ShareItem>,
    pub address: String,
    pub address_fallback: bool,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub canceled: bool,
}
