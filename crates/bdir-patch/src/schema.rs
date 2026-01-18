use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchV1 {
    pub v: u8,
    /// Optional page-level hash binding.
    ///
    /// When present, validators MUST reject the patch if the target document/edit-packet
    /// page hash does not exactly match this value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<String>,
    pub ops: Vec<PatchOpV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpType {
    Replace,
    Delete,
    InsertAfter,
    Suggest,
}

/// Explicit delete semantics.
///
/// The protocol historically treated delete as "remove all occurrences".
/// This enum makes intent explicit and reviewable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeleteOccurrence {
    /// Remove only the first occurrence of `before`.
    First,
    /// Remove all occurrences of `before`.
    All,
}

/// Patch operation in RFC-0001 v1 wire format.
///
/// Field naming:
/// - `blockId` in JSON, `block_id` in Rust.
/// - `content` is used for `insert_after`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOpV1 {
    pub op: OpType,

    #[serde(rename = "blockId")]
    pub block_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// Delete occurrence semantics (required for `delete`).
    ///
    /// JSON field name: `occurrence`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence: Option<DeleteOccurrence>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
