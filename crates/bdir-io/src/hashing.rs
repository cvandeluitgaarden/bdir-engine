//! Hash helpers for canonical JSON and cache keys.

use serde::Serialize;

use crate::canonical_json::to_canonical_json_bytes;

/// Return lowercase hex SHA-256 of bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Hash canonical JSON bytes using SHA-256 and return lowercase hex.
pub fn sha256_canonical_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = to_canonical_json_bytes(value)?;
    Ok(sha256_hex(&bytes))
}

/// Compute a deterministic cache key.
///
/// This follows the guidance in `docs/caching.md`:
///   bdir-patch|model=<...>|prompt=<...>|schema=v1|packet=sha256:<...>
pub fn cache_key_v1(
    model_id: &str,
    prompt_version: &str,
    packet: &impl Serialize,
) -> Result<String, serde_json::Error> {
    let packet_hash = sha256_canonical_json(packet)?;
    Ok(format!(
        "bdir-patch|model={model_id}|prompt={prompt_version}|schema=v1|packet=sha256:{packet_hash}"
    ))
}
