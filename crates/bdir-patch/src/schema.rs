use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchV1 {
    pub v: u8,
    pub ops: Vec<PatchOpV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpType {
    Replace,
    Delete,
    InsertAfter,
    Suggest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOpV1 {
    pub op: OpType,
    pub block_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
