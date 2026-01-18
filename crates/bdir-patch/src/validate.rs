use bdir_core::model::Document;

use crate::{EditPacketV1, schema::{OpType, PatchV1}};

/// Validate a patch against a document. Strict and fail-fast.
///
/// Rules:
/// - patch version must be supported
/// - block_id must exist
/// - required fields must be present per op
/// - `before` (when required) must be found in the block text
/// - optional guard: reject very short `before` strings (ambiguity)
pub fn validate_patch(doc: &Document, patch: &PatchV1) -> Result<(), String> {
    if patch.v != 1 {
        return Err(format!("unsupported patch version {}", patch.v));
    }

    // Optional safety binding: ensure the patch is only applied to the intended page version.
    if let Some(expected) = patch.h.as_deref() {
        if doc.page_hash != expected {
            return Err(format!(
                "patch page hash mismatch (expected '{}', got '{}')",
                expected, doc.page_hash
            ));
        }
    }

    for (i, op) in patch.ops.iter().enumerate() {
        let block = doc
            .blocks
            .iter()
            .find(|b| b.id == op.block_id)
            .ok_or_else(|| format!("ops[{i}] references unknown block_id '{}'", op.block_id))?;

        match op.op {
            OpType::Replace => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (replace) missing before"))?;
                let _after = op
                    .after
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (replace) missing after"))?;

                guard_before(i, before)?;
                if !block.text.contains(before) {
                    return Err(format!(
                        "ops[{i}] (replace) before substring not found in block '{}'",
                        op.block_id
                    ));
                }
            }

            OpType::Delete => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (delete) missing before"))?;

                guard_before(i, before)?;
                if !block.text.contains(before) {
                    return Err(format!(
                        "ops[{i}] (delete) before substring not found in block '{}'",
                        op.block_id
                    ));
                }
            }

            OpType::InsertAfter => {
                let _content = op
                    .content
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (insert_after) missing content"))?;
                // No `before` required; insertion is anchored by block_id + position.
                if _content.trim().is_empty() {
                    return Err(format!("ops[{i}] (insert_after) content is empty"));
                }
            }

            OpType::Suggest => {
                let msg = op
                    .message
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (suggest) missing message"))?;
                if msg.trim().is_empty() {
                    return Err(format!("ops[{i}] (suggest) message is empty"));
                }
            }
        }
    }

    Ok(())
}

fn guard_before(i: usize, before: &str) -> Result<(), String> {
    const MIN_BEFORE_LEN: usize = 8;

    if before.trim().is_empty() {
        return Err(format!("ops[{i}] before is empty/whitespace"));
    }
    if before.chars().count() < MIN_BEFORE_LEN {
        return Err(format!(
            "ops[{i}] before is too short (<{MIN_BEFORE_LEN} chars); likely ambiguous"
        ));
    }
    Ok(())
}

pub fn validate_patch_against_edit_packet(packet: &EditPacketV1, patch: &PatchV1) -> Result<(), String> {
    if patch.v != 1 {
        return Err(format!("unsupported patch version {}", patch.v));
    }
    if packet.v != 1 {
        return Err(format!("unsupported edit packet version {}", packet.v));
    }

    // Optional safety binding: ensure the patch is only applied to the intended page version.
    if let Some(expected) = patch.h.as_deref() {
        if packet.h != expected {
            return Err(format!(
                "patch page hash mismatch (expected '{}', got '{}')",
                expected, packet.h
            ));
        }
    }

    for (i, op) in patch.ops.iter().enumerate() {
        let block = packet
            .b
            .iter()
            .find(|t| t.0 == op.block_id)
            .ok_or_else(|| format!("ops[{i}] references unknown block_id '{}'", op.block_id))?;

        // tuple layout: (id, kind, text_hash, text)
        let block_text = &block.3;

        match op.op {
            OpType::Replace => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (replace) missing before"))?;
                let _after = op
                    .after
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (replace) missing after"))?;

                guard_before(i, before)?;
                if !block_text.contains(before) {
                    return Err(format!(
                        "ops[{i}] (replace) before substring not found in block '{}'",
                        op.block_id
                    ));
                }
            }

            OpType::Delete => {
                let before = op
                    .before
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (delete) missing before"))?;

                guard_before(i, before)?;
                if !block_text.contains(before) {
                    return Err(format!(
                        "ops[{i}] (delete) before substring not found in block '{}'",
                        op.block_id
                    ));
                }
            }

            OpType::InsertAfter => {
                let content = op
                    .content
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (insert_after) missing content"))?;
                if content.trim().is_empty() {
                    return Err(format!("ops[{i}] (insert_after) content is empty"));
                }
            }

            OpType::Suggest => {
                let msg = op
                    .message
                    .as_deref()
                    .ok_or_else(|| format!("ops[{i}] (suggest) missing message"))?;
                if msg.trim().is_empty() {
                    return Err(format!("ops[{i}] (suggest) message is empty"));
                }
            }
        }
    }

    Ok(())
}