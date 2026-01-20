//! Helpers for parsing `bdir-core::Document` JSON with improved diagnostics.
//!
//! Motivation: serde's default "missing field X" error is technically correct
//! but often unhelpful for users generating fixtures or integrating with the
//! engine. These helpers keep strict validation behavior unchanged while
//! providing actionable messages about required top-level fields.

use std::fmt;

use bdir_core::model::Document;
use bdir_core::hash::hash_hex;
use serde::de::Error as _;
use serde_json::Value;

const REQUIRED_TOP_LEVEL_FIELDS: &[&str] = &["hash_algorithm", "blocks"];

/// A structured error for parsing a Document JSON payload.
#[derive(Debug)]
pub enum DocumentJsonError {
    /// The input was not valid JSON.
    InvalidJson(serde_json::Error),
    /// The input JSON was valid, but missing required top-level fields.
    MissingRequiredTopLevelFields {
        missing: Vec<&'static str>,
        required: Vec<&'static str>,
    },
    /// JSON was valid, but did not match the Document schema/shape.
    InvalidDocumentShape(serde_json::Error),

    /// Document JSON was valid, but declared an unsupported `hash_algorithm`.
    UnsupportedHashAlgorithm(String),
}

impl fmt::Display for DocumentJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentJsonError::InvalidJson(e) => {
                write!(f, "Invalid JSON: {e}")
            }
            DocumentJsonError::MissingRequiredTopLevelFields { missing, required } => {
                write!(
                    f,
                    "Invalid Document JSON: missing required top-level field(s): {}. Required top-level fields: {}.",
                    missing.join(", "),
                    required.join(", ")
                )
            }
            DocumentJsonError::InvalidDocumentShape(e) => {
                // Include a stable hint about required fields, but keep the original
                // serde message (it is often the most specific info available).
                write!(
                    f,
                    "Invalid Document JSON shape: {e}. Required top-level fields: {}.",
                    REQUIRED_TOP_LEVEL_FIELDS.join(", ")
                )
            }

            DocumentJsonError::UnsupportedHashAlgorithm(algo) => {
                write!(
                    f,
                    "Unsupported hash_algorithm '{algo}'. Supported algorithms: sha256, xxh64."
                )
            }
        }
    }
}

impl std::error::Error for DocumentJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DocumentJsonError::InvalidJson(e) => Some(e),
            DocumentJsonError::InvalidDocumentShape(e) => Some(e),
            DocumentJsonError::MissingRequiredTopLevelFields { .. } => None,
            DocumentJsonError::UnsupportedHashAlgorithm(_) => None,
        }
    }
}

/// Parse a Document JSON string with improved diagnostics for missing required
/// top-level fields.
///
/// Strictness is unchanged: missing required fields still fails.
pub fn parse_document_json_str(s: &str) -> Result<Document, DocumentJsonError> {
    let v: Value = serde_json::from_str(s).map_err(DocumentJsonError::InvalidJson)?;
    let obj = v
        .as_object()
        .ok_or_else(|| {
            DocumentJsonError::InvalidDocumentShape(serde_json::Error::custom("expected a JSON object"))
        })?;

    let mut missing: Vec<&'static str> = Vec::new();
    for &k in REQUIRED_TOP_LEVEL_FIELDS {
        if !obj.contains_key(k) {
            missing.push(k);
        }
    }
    if !missing.is_empty() {
        return Err(DocumentJsonError::MissingRequiredTopLevelFields {
            missing,
            required: REQUIRED_TOP_LEVEL_FIELDS.to_vec(),
        });
    }

    let mut doc: Document = serde_json::from_value(v).map_err(DocumentJsonError::InvalidDocumentShape)?;

    // RFC-0001: receivers MUST reject unrecognized hash algorithms.
    let algo = doc.hash_algorithm.trim().to_lowercase();
    if algo.is_empty() || hash_hex(&algo, "").is_none() {
        return Err(DocumentJsonError::UnsupportedHashAlgorithm(doc.hash_algorithm));
    }
    doc.hash_algorithm = algo;

    Ok(doc)
}
