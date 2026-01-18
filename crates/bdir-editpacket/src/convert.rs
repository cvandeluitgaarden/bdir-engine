use bdir_core::model::Document;

use crate::schema::{BlockTupleV1, EditPacketV1};

/// Convert a core Document into an ultra-min Edit Packet (v1).
pub fn from_document(doc: &Document, tid: Option<String>) -> EditPacketV1 {
    let blocks: Vec<BlockTupleV1> = doc
        .blocks
        .iter()
        .map(|b| (b.id.clone(), b.kind_code, b.text_hash.clone(), b.text.clone()))
        .collect();

    EditPacketV1 {
        v: 1,
        tid,
        h: doc.page_hash.clone(),
        ha: doc.hash_algorithm.clone(),
        b: blocks,
    }
}
