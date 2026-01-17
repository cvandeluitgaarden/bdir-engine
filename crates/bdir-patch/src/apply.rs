use crate::schema::{OpType, PatchV1};
use crate::validate::validate_patch_against_edit_packet;
use bdir_core::hash::{canonicalize_text, xxh64_hex};
use bdir_editpacket::{BlockTupleV1, EditPacketV1};

/// Apply a patch against an Edit Packet and return an updated Edit Packet.
///
/// Deterministic semantics:
/// - replace: replace the FIRST occurrence of `before` with `after` within the block text
/// - delete: remove ALL occurrences of `before` within the block text (substring delete)
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
    // Validate first (stable error messages come from validator).
    validate_patch_against_edit_packet(packet, patch)?;

    // For now we only support xxh64; you can extend later.
    let algo = packet.ha.as_deref().unwrap_or("xxh64");
    if algo != "xxh64" {
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
                let next_text = replace_first(&current_text, before, after);
                out.b[idx].3 = next_text;
            }

            OpType::Delete => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| "ops delete missing before (should be validated)".to_string())?;

                let idx = find_block_index(&out.b, &op.block_id)
                    .ok_or_else(|| format!("unknown block_id '{}'", op.block_id))?;

                let current_text = out.b[idx].3.clone();
                out.b[idx].3 = current_text.replace(before, "");
            }

            OpType::InsertAfter => {
                let after = op
                    .after
                    .as_deref()
                    .ok_or_else(|| "ops insert_after missing after (should be validated)".to_string())?;

                let anchor_idx = find_block_index(&out.b, &op.block_id)
                    .ok_or_else(|| format!("unknown block_id '{}'", op.block_id))?;

                // Inherit kindCode from anchor for now (simple + deterministic).
                let anchor_kind = out.b[anchor_idx].1;

                // Create deterministic id: "<anchor>_ins", "<anchor>_ins2", ...
                let new_id = make_insert_id(&out.b, &op.block_id);

                // Placeholder hash, recomputed at the end.
                let new_tuple: BlockTupleV1 = (new_id, anchor_kind, String::new(), after.to_string());

                out.b.insert(anchor_idx + 1, new_tuple);
            }

            OpType::Suggest => {
                // Non-mutating. Validation already ensures non-empty `message`.
            }
        }
    }

    // Recompute hashes after applying all ops.
    recompute_edit_packet_hashes(&mut out);

    Ok(out)
}

fn find_block_index(blocks: &[BlockTupleV1], block_id: &str) -> Option<usize> {
    blocks.iter().position(|t| t.0 == block_id)
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

/// Deterministic inserted id: "<anchor>_ins", or "<anchor>_ins2", "_ins3", ...
fn make_insert_id(blocks: &[BlockTupleV1], anchor_id: &str) -> String {
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

/// Recompute block text hashes and packet hash `h` using xxh64.
///
/// Packet hash input is identical to the Document hash payload you used earlier:
/// `{blockId}\t{kindCode}\t{textHash}\n` for each block in order.
fn recompute_edit_packet_hashes(packet: &mut EditPacketV1) {
    packet.ha = Some("xxh64".to_string());

    // Recompute each block's textHash from canonicalized text.
    for t in &mut packet.b {
        let canon = canonicalize_text(&t.3);
        t.2 = xxh64_hex(&canon);
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

    packet.h = xxh64_hex(&payload);
}
