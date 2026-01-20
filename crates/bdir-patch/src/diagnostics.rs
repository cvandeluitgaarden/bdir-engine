use crate::schema::OpType;

use serde::{Deserialize, Serialize};

/// Stable, machine-readable diagnostic codes for patch validation.
///
/// These codes are intended for programmatic handling (CI, tooling, UI), while
/// `message` remains human-oriented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnsupportedPatchVersion,
    UnsupportedEditPacketVersion,
    PatchPageHashMismatch,
    PatchPageHashMissing,
    /// Patch hash algorithm (`ha`) does not match the target document/packet algorithm.
    HashAlgorithmMismatch,
    /// insert_after `new_block_id` conflicts with an existing block id.
    DuplicateBlockId,
    UnknownBlockId,
    MissingField,
    UnexpectedField,
    BeforeEmpty,
    BeforeTooShort,
    BeforeNotFound,
    /// `before` matched more than once but no `occurrence` was provided.
    BeforeAmbiguous,
    /// `occurrence` was provided but is invalid or out of range.
    OccurrenceOutOfRange,
    /// The target block's kindCode is not allowed under strict kindCode policy enforcement.
    KindCodeDisallowed,
    /// The edit packet's kindCode is outside RFC-0001 v1 canonical importance ranges.
    KindCodeOutOfRange,
    ContentEmpty,
    MessageEmpty,
}

/// A single validation diagnostic.
///
/// Designed to be:
/// - stable enough for machine handling (via `code`, `path`)
/// - still useful to humans (via `message`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationDiagnostic {
    pub code: DiagnosticCode,
    /// JSON-ish path such as `v`, `h`, `ops[3].before`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<OpType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    pub message: String,
}

/// Structured error wrapper for validation failures.
///
/// The validator is currently fail-fast and returns a single diagnostic, but the
/// container supports multiple diagnostics to allow future expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    pub diagnostics: Vec<ValidationDiagnostic>,
}

impl ValidationError {
    pub fn single(diag: ValidationDiagnostic) -> Self {
        Self {
            diagnostics: vec![diag],
        }
    }

    /// Backward-compatible string form used by existing call sites.
    ///
    /// This returns the first diagnostic's message (or a generic fallback).
    pub fn legacy_message(&self) -> String {
        self.diagnostics
            .first()
            .map(|d| d.message.clone())
            .unwrap_or_else(|| "validation failed".to_string())
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.legacy_message())
    }
}

impl std::error::Error for ValidationError {}
