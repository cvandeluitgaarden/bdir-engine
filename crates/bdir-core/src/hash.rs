use xxhash_rust::xxh3::xxh3_64;

/// Canonicalize text for hashing.
///
/// Goals:
/// - Deterministic across platforms (normalize CRLF -> LF)
/// - Avoid hash churn from trailing whitespace differences (optional but useful)
///
/// Notes:
/// - We do NOT change internal whitespace, punctuation, or casing.
/// - We do NOT trim leading whitespace (could be meaningful in Markdown/code).
pub fn canonicalize_text(input: &str) -> String {
    // Normalize newlines
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");

    // Trim trailing whitespace on each line (keeps content stable across editors)
    // Important: preserve final newline presence as-is (we don't force-add/remove).
    let mut out = String::with_capacity(normalized.len());

    // We can't use lines() because it drops trailing empty last line info.
    // So we iterate manually while preserving '\n' exactly.
    for segment in normalized.split_inclusive('\n') {
        if let Some(stripped) = segment.strip_suffix('\n') {
            out.push_str(stripped.trim_end_matches(|c: char| c == ' ' || c == '\t'));
            out.push('\n');
        } else {
            // Last segment (no trailing '\n')
            out.push_str(segment.trim_end_matches(|c: char| c == ' ' || c == '\t'));
        }
    }

    out
}

/// Compute an xxh64-style hash (hex) over UTF-8 bytes.
///
/// Implementation detail:
/// - Uses xxh3_64 (from `xxhash-rust`) for speed and stability.
/// - Returned as fixed-width 16-char lowercase hex.
pub fn xxh64_hex(input: &str) -> String {
    format!("{:016x}", xxh3_64(input.as_bytes()))
}

/// Convenience: hash canonicalized text.
pub fn xxh64_canon_hex(input: &str) -> String {
    let canon = canonicalize_text(input);
    xxh64_hex(&canon)
}
