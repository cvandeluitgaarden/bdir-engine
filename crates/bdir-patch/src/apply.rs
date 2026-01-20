use crate::schema::{DeleteOccurrence, Occurrence, OpType, PatchV1};
use crate::telemetry::PatchTelemetry;
use crate::validate::{
    validate_patch,
    validate_patch_with_options,
    validate_patch_against_edit_packet_with_options,
    ValidateOptions,
};
use bdir_core::hash::{hash_canon_hex, hash_hex};
use bdir_core::model::{Block, Document};
use bdir_editpacket::{BlockTupleV1, EditPacketV1};

/// Apply a patch against an Edit Packet and return an updated Edit Packet.
///
/// Deterministic semantics:
/// - replace: replace an unambiguous match of `before` with `after` (or a selected `occurrence`)
/// - delete: delete an unambiguous match of `before` (or a selected `occurrence`)
/// - insert_after: inserts a new block AFTER the referenced block_id, with `text = after`
/// - suggest: no mutation (informational only)
///
/// Safety:
/// - Calls `validate_patch_against_edit_packet()` first.
/// - Recomputes all `textHash` values and the packet hash `h` after applying.
pub fn apply_patch_against_edit_packet(
    packet: &EditPacketV1,
    patch: &PatchV1,
) -> Result<EditPacketV1, String> {
    apply_patch_against_edit_packet_with_options(packet, patch, ValidateOptions::default())
}

/// Apply a patch against an Edit Packet with configurable validator options.
///
/// This enables optional strict validation (e.g. kindCode policy enforcement) while keeping
/// the default behavior unchanged.
pub fn apply_patch_against_edit_packet_with_options(
    packet: &EditPacketV1,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> Result<EditPacketV1, String> {
    // Validate first (stable error messages come from validator).
    validate_patch_against_edit_packet_with_options(packet, patch, opts)?;

    // Support any algorithm implemented by bdir-core.
    let algo = packet.ha.as_str();
    if hash_hex(algo, "").is_none() {
        return Err(format!("unsupported hash algorithm '{algo}'"));
    }

    let mut out = packet.clone();

    for op in &patch.ops {
        match op.op {
            OpType::Replace => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| "ops replace missing before (should be validated)".to_string())?;
                let after = op
                    .after
                    .as_deref()
                    .ok_or_else(|| "ops replace missing after (should be validated)".to_string())?;

                let idx = find_block_index(&out.b, &op.block_id)
                    .ok_or_else(|| format!("unknown block_id '{}'", op.block_id))?;

                let current_text = out.b[idx].3.clone();
                let next_text = match op.occurrence {
                    Some(Occurrence::Index(n)) => replace_nth_non_overlapping(&current_text, before, after, n.try_into().unwrap())
                        .ok_or_else(|| {
                            format!(
                                "replace occurrence out of range (block_id='{}', occurrence={n})",
                                op.block_id
                            )
                        })?,
                    Some(Occurrence::Legacy(_)) => {
                        return Err("replace occurrence must be an integer (legacy string values are delete-only)".to_string());
                    }
                    None => replace_first(&current_text, before, after),
                };
                out.b[idx].3 = next_text;
            }

            OpType::Delete => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| "ops delete missing before (should be validated)".to_string())?;

                let occ = op.occurrence;

                let idx = find_block_index(&out.b, &op.block_id)
                    .ok_or_else(|| format!("unknown block_id '{}'", op.block_id))?;

                let current_text = out.b[idx].3.clone();
                out.b[idx].3 = match occ {
                    Some(Occurrence::Legacy(DeleteOccurrence::All)) => current_text.replace(before, ""),
                    Some(Occurrence::Legacy(DeleteOccurrence::First)) => delete_first(&current_text, before),
                    Some(Occurrence::Index(n)) => delete_nth_non_overlapping(&current_text, before, n.try_into().unwrap()).ok_or_else(|| {
                        format!(
                            "delete occurrence out of range (block_id='{}', occurrence={n})",
                            op.block_id
                        )
                    })?,
                    None => delete_first(&current_text, before),
                };
            }

            OpType::InsertAfter => {
                let content = op
                    .content
                    .as_deref()
                    .ok_or_else(|| "ops insert_after missing content (should be validated)".to_string())?;

                let anchor_idx = find_block_index(&out.b, &op.block_id)
                    .ok_or_else(|| format!("unknown block_id '{}'", op.block_id))?;

                // Inherit kindCode from anchor for now (simple + deterministic).
                let anchor_kind = out.b[anchor_idx].1;

                // Create deterministic id: "<anchor>_ins", "<anchor>_ins2", ...
                let new_id = make_insert_id_editpacket(&out.b, &op.block_id);

                // Placeholder hash, recomputed at the end.
                let new_tuple: BlockTupleV1 = (new_id, anchor_kind, String::new(), content.to_string());

                out.b.insert(anchor_idx + 1, new_tuple);
            }

            OpType::Suggest => {
                // Non-mutating. Validation already ensures non-empty `message`.
            }
        }
    }

    // Recompute hashes after applying all ops.
    recompute_edit_packet_hashes(&mut out, algo);

    Ok(out)
}

/// Apply a patch against a full Document and return an updated Document.
///
/// This is the CLI/workflow-friendly variant used when downstream systems
/// need an updated Document JSON for renderers.
///
/// Semantics match `apply_patch_against_edit_packet`.
///
/// Safety:
/// - Calls `validate_patch()` first.
/// - Recomputes block `text_hash` values and `page_hash` after applying.
pub fn apply_patch_against_document(doc: &Document, patch: &PatchV1) -> Result<Document, String> {
    // Validate first (stable error messages come from validator).
    validate_patch(doc, patch)?;

    let mut out = doc.clone();

    for op in &patch.ops {
        match op.op {
            OpType::Replace => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| "ops replace missing before (should be validated)".to_string())?;
                let after = op
                    .after
                    .as_deref()
                    .ok_or_else(|| "ops replace missing after (should be validated)".to_string())?;

                let idx = find_doc_block_index(&out.blocks, &op.block_id)
                    .ok_or_else(|| format!("unknown block_id '{}'", op.block_id))?;

                let current_text = out.blocks[idx].text.clone();
                out.blocks[idx].text = match op.occurrence {
                    Some(Occurrence::Index(n)) => replace_nth_non_overlapping(&current_text, before, after, n.try_into().unwrap())
                        .ok_or_else(|| {
                            format!(
                                "replace occurrence out of range (block_id='{}', occurrence={n})",
                                op.block_id
                            )
                        })?,
                    Some(Occurrence::Legacy(_)) => {
                        return Err("replace occurrence must be an integer (legacy string values are delete-only)".to_string());
                    }
                    None => replace_first(&current_text, before, after),
                };
            }

            OpType::Delete => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| "ops delete missing before (should be validated)".to_string())?;

                let occ = op.occurrence;

                let idx = find_doc_block_index(&out.blocks, &op.block_id)
                    .ok_or_else(|| format!("unknown block_id '{}'", op.block_id))?;

                let current_text = out.blocks[idx].text.clone();
                out.blocks[idx].text = match occ {
                    Some(Occurrence::Legacy(DeleteOccurrence::All)) => current_text.replace(before, ""),
                    Some(Occurrence::Legacy(DeleteOccurrence::First)) => delete_first(&current_text, before),
                    Some(Occurrence::Index(n)) => delete_nth_non_overlapping(&current_text, before, n.try_into().unwrap()).ok_or_else(|| {
                        format!(
                            "delete occurrence out of range (block_id='{}', occurrence={n})",
                            op.block_id
                        )
                    })?,
                    None => delete_first(&current_text, before),
                };
            }

            OpType::InsertAfter => {
                let content = op
                    .content
                    .as_deref()
                    .ok_or_else(|| "ops insert_after missing content (should be validated)".to_string())?;

                let anchor_idx = find_doc_block_index(&out.blocks, &op.block_id)
                    .ok_or_else(|| format!("unknown block_id '{}'", op.block_id))?;

                let anchor_kind = out.blocks[anchor_idx].kind_code;

                let new_id = make_insert_id_doc(&out.blocks, &op.block_id);
                let new_block = Block {
                    id: new_id,
                    kind_code: anchor_kind,
                    text_hash: String::new(),
                    text: content.to_string(),
                };

                out.blocks.insert(anchor_idx + 1, new_block);
            }

            OpType::Suggest => {
                // Non-mutating.
            }
        }
    }

    // Recompute hashes after applying all ops (respects doc.hash_algorithm).
    out.recompute_hashes();

    Ok(out)
}

/// Apply a patch against a Document using custom validation options.
pub fn apply_patch_against_document_with_options(
    doc: &Document,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> Result<Document, String> {
    validate_patch_with_options(doc, patch, opts)?;
    apply_patch_against_document(doc, patch)
}

fn find_block_index(blocks: &[BlockTupleV1], block_id: &str) -> Option<usize> {
    blocks.iter().position(|t| t.0 == block_id)
}

fn find_doc_block_index(blocks: &[Block], block_id: &str) -> Option<usize> {
    blocks.iter().position(|b| b.id == block_id)
}

/// Replace only the FIRST occurrence (deterministic).
fn replace_first(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_string();
    }

    match haystack.find(needle) {
        None => haystack.to_string(),
        Some(pos) => {
            let mut out = String::with_capacity(
                haystack.len().saturating_sub(needle.len()) + replacement.len(),
            );
            out.push_str(&haystack[..pos]);
            out.push_str(replacement);
            out.push_str(&haystack[pos + needle.len()..]);
            out
        }
    }
}

/// Delete only the FIRST occurrence (deterministic).
fn delete_first(haystack: &str, needle: &str) -> String {
    replace_first(haystack, needle, "")
}

/// Deterministic inserted id: "<anchor>_ins", or "<anchor>_ins2", "_ins3", ...
fn make_insert_id_editpacket(blocks: &[BlockTupleV1], anchor_id: &str) -> String {
    let base = format!("{anchor_id}_ins");

    if !blocks.iter().any(|t| t.0 == base) {
        return base;
    }

    for n in 2u32.. {
        let candidate = format!("{base}{n}");
        if !blocks.iter().any(|t| t.0 == candidate) {
            return candidate;
        }
    }

    base
}

/// Deterministic inserted id: "<anchor>_ins", or "<anchor>_ins2", "_ins3", ...
fn make_insert_id_doc(blocks: &[Block], anchor_id: &str) -> String {
    let base = format!("{anchor_id}_ins");

    if !blocks.iter().any(|b| b.id == base) {
        return base;
    }

    for n in 2u32.. {
        let candidate = format!("{base}{n}");
        if !blocks.iter().any(|b| b.id == candidate) {
            return candidate;
        }
    }

    base
}

/// Recompute block text hashes and packet hash `h`.
///
/// Packet hash input is identical to the Document hash payload:
/// `{block_id}\t{kind_code}\t{text_hash}\n` for each block in order.
fn recompute_edit_packet_hashes(packet: &mut EditPacketV1, algo: &str) {
    // Preserve the declared algorithm (and ensure hashes align with it).
    packet.ha = algo.to_string();

    // Recompute each block's textHash from canonicalized text.
    for t in &mut packet.b {
        t.2 = hash_canon_hex(algo, &t.3).expect("supported algorithm");
    }

    // Recompute packet hash from ordered tuples.
    let mut payload = String::new();
    for t in &packet.b {
        payload.push_str(&t.0);
        payload.push('\t');
        payload.push_str(&t.1.to_string());
        payload.push('\t');
        payload.push_str(&t.2);
        payload.push('\n');
    }

    packet.h = hash_hex(algo, &payload).expect("supported algorithm");
}

// -----------------------------------------------------------------------------
// Telemetry wrappers (deterministic)
// -----------------------------------------------------------------------------

/// Apply a patch against an Edit Packet and return deterministic telemetry.
///
/// Returns a tuple of (result, telemetry) so callers can emit telemetry even on failure.
pub fn apply_patch_against_edit_packet_with_telemetry(
    packet: &EditPacketV1,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> (Result<EditPacketV1, String>, PatchTelemetry) {
    use std::time::Instant;

    let start = Instant::now();
    let (patch_ops, patch_ops_by_type, target_blocks) = PatchTelemetry::op_counts(&patch.ops);

    let input_chars = Some(packet.b.iter().map(|t| t.3.len()).sum::<usize>());

    let res = apply_patch_against_edit_packet_with_options(packet, patch, opts.clone());
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let output_chars = res
        .as_ref()
        .ok()
        .map(|p| p.b.iter().map(|t| t.3.len()).sum::<usize>());

    let tel = PatchTelemetry {
        op: "apply".to_string(),
        ok: res.is_ok(),
        elapsed_ms,
        patch_v: patch.v as u16,
        edit_packet_v: Some(packet.v as u16),
        hash_algorithm: Some(packet.ha.clone()),
        patch_ops,
        patch_ops_by_type,
        target_blocks,
        strict_kind_code: opts.strict_kind_code,
        min_before_len: opts.min_before_len,
        kind_code_allow: if opts.strict_kind_code {
            PatchTelemetry::kind_allow_strings(&opts.kind_code_policy.allow_ranges)
        } else {
            vec![]
        },
        input_chars,
        output_chars,
        error_code: res.as_ref().err().map(|_| "apply_failed".to_string()),
    };

    (res, tel)
}

/// Apply a patch against a Document and return deterministic telemetry.
///
/// Returns a tuple of (result, telemetry) so callers can emit telemetry even on failure.
pub fn apply_patch_against_document_with_telemetry(
    doc: &Document,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> (Result<Document, String>, PatchTelemetry) {
    use std::time::Instant;

    let start = Instant::now();
    let (patch_ops, patch_ops_by_type, target_blocks) = PatchTelemetry::op_counts(&patch.ops);

    let input_chars = Some(doc.blocks.iter().map(|b| b.text.len()).sum::<usize>());

    let res = apply_patch_against_document_with_options(doc, patch, opts.clone());
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let output_chars = res
        .as_ref()
        .ok()
        .map(|d| d.blocks.iter().map(|b| b.text.len()).sum::<usize>());

    let tel = PatchTelemetry {
        op: "apply".to_string(),
        ok: res.is_ok(),
        elapsed_ms,
        patch_v: patch.v as u16,
        edit_packet_v: None,
        hash_algorithm: Some(doc.hash_algorithm.clone()),
        patch_ops,
        patch_ops_by_type,
        target_blocks,
        strict_kind_code: opts.strict_kind_code,
        min_before_len: opts.min_before_len,
        kind_code_allow: if opts.strict_kind_code {
            PatchTelemetry::kind_allow_strings(&opts.kind_code_policy.allow_ranges)
        } else {
            vec![]
        },
        input_chars,
        output_chars,
        error_code: res.as_ref().err().map(|_| "apply_failed".to_string()),
    };

    (res, tel)
}

/// Replace the Nth non-overlapping occurrence (1-indexed) of `before` with `after`.
/// Returns None if the Nth occurrence does not exist.
pub fn replace_nth_non_overlapping(haystack: &str, before: &str, after: &str, n: usize) -> Option<String> {
    if before.is_empty() || n == 0 {
        return None;
    }

    let mut start = 0usize;
    let mut count = 0usize;

    while let Some(rel) = haystack[start..].find(before) {
        let idx = start + rel;
        count += 1;

        if count == n {
            let mut out = String::with_capacity(haystack.len() - before.len() + after.len());
            out.push_str(&haystack[..idx]);
            out.push_str(after);
            out.push_str(&haystack[idx + before.len()..]);
            return Some(out);
        }

        // Non-overlapping: jump past this match
        start = idx + before.len();
        if start > haystack.len() {
            break;
        }
    }

    None
}

pub fn delete_nth_non_overlapping(haystack: &str, before: &str, n: usize) -> Option<String> {
    replace_nth_non_overlapping(haystack, before, "", n)
}
