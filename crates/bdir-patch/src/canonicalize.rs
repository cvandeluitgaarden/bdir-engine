//! Patch operation canonicalization.
//!
//! The protocol's deterministic guarantees benefit when patch `ops` ordering
//! is canonicalized before hashing, comparing, or displaying patches.
//!
//! NOTE: This does **not** change validation rules. It is a pure reordering.

use std::collections::HashMap;

use bdir_editpacket::EditPacketV1;

use crate::schema::{DeleteOccurrence, Occurrence, OpType, PatchOpV1, PatchV1};

/// Options for canonicalizing patch operation ordering.
#[derive(Debug, Clone, Copy)]
pub struct CanonicalizeOptions {
    /// When true, operation ordering is derived from Edit Packet block order.
    ///
    /// If false, ordering falls back to lexicographic `block_id` ordering.
    pub prefer_edit_packet_order: bool,
}

impl Default for CanonicalizeOptions {
    fn default() -> Self {
        Self { prefer_edit_packet_order: true }
    }
}

/// Canonicalize patch operation ordering without any document context.
///
/// Ordering:
/// 1) `block_id` (lexicographic)
/// 2) operation type (delete, replace, insert_after, suggest)
/// 3) operation-specific fields (`before`, `after`, insert_after fields, `message`, `occurrence`)
/// 4) original index (tie-breaker for deterministic output)
pub fn canonicalize_patch_ops(patch: &mut PatchV1) {
    canonicalize_ops_inner(&mut patch.ops, None);
}

/// Canonicalize patch operation ordering using Edit Packet block order.
///
/// This produces the most human-friendly and stable ordering because it matches
/// the document's natural reading order.
pub fn canonicalize_patch_ops_against_edit_packet(packet: &EditPacketV1, patch: &mut PatchV1) {
    let mut idx = HashMap::with_capacity(packet.b.len());
    for (i, t) in packet.b.iter().enumerate() {
        idx.insert(t.0.as_str(), i as i64);
    }
    canonicalize_ops_inner(&mut patch.ops, Some(&idx));
}

fn op_rank(op: OpType) -> i32 {
    match op {
        OpType::Delete => 0,
        OpType::Replace => 1,
        OpType::InsertAfter => 2,
        OpType::Suggest => 3,
    }
}

fn occurrence_rank(o: Option<Occurrence>) -> i64 {
    match o {
        // Canonical RFC form: 1-indexed.
        Some(Occurrence::Index(n)) => n as i64,
        // Legacy delete forms: keep deterministic ordering.
        Some(Occurrence::Legacy(DeleteOccurrence::First)) => 1,
        Some(Occurrence::Legacy(DeleteOccurrence::All)) => i64::MAX - 1,
        None => i64::MAX,
    }
}

fn canonicalize_ops_inner(ops: &mut Vec<PatchOpV1>, order: Option<&HashMap<&str, i64>>) {
    // Sort a derived vector of indices so we can include the original index as a tie-breaker.
    // This ensures deterministic output even though Rust's `sort_by` is not stable.
    let mut orderings: Vec<(usize, CanonicalKey)> = ops
        .iter()
        .enumerate()
        .map(|(i, op)| {
            let block_pos = order
                .and_then(|m| m.get(op.block_id.as_str()).copied())
                .unwrap_or(i64::MAX);

            (
                i,
                CanonicalKey {
                    block_pos,
                    block_id: op.block_id.clone(),
                    op_rank: op_rank(op.op),
                    before: op.before.clone().unwrap_or_default(),
                    after: op.after.clone().unwrap_or_default(),
                    insert_new_block_id: op.new_block_id.clone().unwrap_or_default(),
                    insert_kind_code: op.kind_code.unwrap_or_default(),
                    insert_text: op.text.clone().unwrap_or_default(),
                    message: op.message.clone().unwrap_or_default(),
                    occurrence_rank: occurrence_rank(op.occurrence),
                },
            )
        })
        .collect();

    orderings.sort_by(|(ai, ak), (bi, bk)| {
        ak.cmp(bk).then_with(|| ai.cmp(bi))
    });

    let mut new_ops = Vec::with_capacity(ops.len());
    for (old_i, _) in orderings {
        new_ops.push(ops[old_i].clone());
    }
    *ops = new_ops;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalKey {
    block_pos: i64,
    block_id: String,
    op_rank: i32,
    before: String,
    after: String,
    insert_new_block_id: String,
    insert_kind_code: u16,
    insert_text: String,
    message: String,
    occurrence_rank: i64,
}

impl Ord for CanonicalKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.block_pos
            .cmp(&other.block_pos)
            .then_with(|| self.block_id.cmp(&other.block_id))
            .then_with(|| self.op_rank.cmp(&other.op_rank))
            .then_with(|| self.before.cmp(&other.before))
            .then_with(|| self.after.cmp(&other.after))
            .then_with(|| self.insert_new_block_id.cmp(&other.insert_new_block_id))
            .then_with(|| self.insert_kind_code.cmp(&other.insert_kind_code))
            .then_with(|| self.insert_text.cmp(&other.insert_text))
            .then_with(|| self.message.cmp(&other.message))
            .then_with(|| self.occurrence_rank.cmp(&other.occurrence_rank))
    }
}

impl PartialOrd for CanonicalKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
