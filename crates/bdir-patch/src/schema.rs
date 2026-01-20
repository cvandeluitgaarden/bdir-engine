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

/// Occurrence selector for `replace` and `delete`.
///
/// RFC-0001 v1.0.2 defines `occurrence` as an optional **integer** (1-indexed)
/// used to disambiguate multiple matches of `before` in a single block.
///
/// Backwards compatibility:
/// - Older engine versions accepted string occurrences for `delete` ("first"/"all").
/// - We continue to accept those spellings on input, but the canonical form is an integer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Occurrence {
    /// Canonical RFC form: 1-indexed occurrence selector.
    Index(u32),
    /// Legacy delete-only semantics.
    Legacy(DeleteOccurrence),
}

/// Patch operation in RFC-0001 v1 wire format.
///
/// Field naming:
/// - Canonical JSON field is `block_id` (snake_case).
/// - For backwards compatibility, `blockId` (camelCase) is accepted on input.
/// - `content` is used for `insert_after`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOpV1 {
    pub op: OpType,

    #[serde(rename = "block_id", alias = "blockId")]
    pub block_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// Occurrence selection for `replace` and `delete`.
    ///
    /// JSON field name: `occurrence`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence: Option<Occurrence>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
