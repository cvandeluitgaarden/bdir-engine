//! Deterministic JSON canonicalization.
//!
//! The goal is stable bytes for hashing and cache keys:
//! - object keys are sorted lexicographically
//! - arrays preserve order
//! - output is minified JSON with no extra whitespace
//!
//! Notes:
//! - Avoid floats in protocol wire types; if floats are introduced later, we must
//!   define normalization rules.

use serde::Serialize;
use serde_json::{Map, Value};

/// Convert a serializable value to canonical JSON bytes.
///
/// Canonicalization rules:
/// - JSON objects are deep-sorted by key
/// - arrays preserve order
/// - scalars are unchanged
/// - output is minified JSON
pub fn to_canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let v = serde_json::to_value(value)?;
    let canon = canonicalize_value(v);
    let mut out = Vec::new();
    serde_json::to_writer(&mut out, &canon)?;
    Ok(out)
}

/// Convert a serializable value to a canonical JSON string.
pub fn to_canonical_json_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = to_canonical_json_bytes(value)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn canonicalize_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));

            let mut new_map = Map::with_capacity(entries.len());
            for (k, v) in entries {
                new_map.insert(k, canonicalize_value(v));
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(canonicalize_value).collect()),
        other => other,
    }
}
