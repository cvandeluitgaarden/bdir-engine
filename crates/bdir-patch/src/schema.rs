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

    /// Hash algorithm identifier for `h`.
    ///
    /// RFC-0001 v1.0.2: if omitted, receivers MUST treat this as "sha256".
    ///
    /// Note: this crate validates `ha` against the target document's declared
    /// `hash_algorithm` to prevent mismatched bindings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ha: Option<String>,

    pub ops: Vec<PatchOpV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpType {
    Replace,
    Delete,
    InsertAfter,
    InsertBefore,
    ReplaceBlock,
    DeleteBlock,
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
///
/// `insert_after` (RFC-0001 v1.0.2) fields:
/// - `new_block_id` (REQUIRED)
/// - `kind_code` (REQUIRED)
/// - `text` (REQUIRED)
///
/// Backwards compatibility:
/// - Older engine versions used `content` instead of `text` and auto-derived
///   `new_block_id`/`kind_code`. Those spellings are accepted on input but are
///   rejected by validation unless the required RFC fields are present.
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

    /// `insert_after` only: identifier for the inserted block.
    #[serde(skip_serializing_if = "Option::is_none", rename = "new_block_id", alias = "newBlockId")]
    pub new_block_id: Option<String>,

    /// `insert_after` only: kind classification for the inserted block.
    #[serde(skip_serializing_if = "Option::is_none", rename = "kind_code", alias = "kindCode")]
    pub kind_code: Option<u16>,

    /// `insert_after` only: canonical text for the inserted block.
    ///
    /// Backwards compatibility: accepts legacy `content` field as an alias.
    #[serde(skip_serializing_if = "Option::is_none", alias = "content")]
    pub text: Option<String>,

    /// `suggest` only: advisory message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Optional human-readable rationale for review/audit.
    ///
    /// RFC-0001 v1.1: advisory only; MUST NOT affect validation or application semantics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// `suggest` only: optional severity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
}
