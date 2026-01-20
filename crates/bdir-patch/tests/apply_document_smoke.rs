use serde_json::json;

use bdir_core::model::Document;
use bdir_patch::{apply::apply_patch_against_document, schema::PatchV1};

#[test]
fn apply_patch_against_document_updates_text_and_hashes() {
    let mut doc: Document = serde_json::from_value(json!({
        "page_hash": "",
        "hash_algorithm": "xxh64",
        "blocks": [
            {"id": "p1", "kind_code": 2, "text_hash": "", "text": "This is teh first paragraph."}
        ]
    }))
    .unwrap();
    doc.recompute_hashes();

    let mut patch: PatchV1 = serde_json::from_value(json!({
        "v": 1,
        "ops": [
            {"op": "replace", "block_id": "p1", "before": "teh first", "after": "the first"}
        ]
    }))
    .unwrap();

    patch.h = Some(doc.page_hash.clone());
    patch.ha = Some(doc.hash_algorithm.clone());

    let updated = apply_patch_against_document(&doc, &patch).unwrap();
    assert!(updated.blocks[0].text.contains("the first paragraph"));
    assert_ne!(updated.page_hash, doc.page_hash);
    assert!(!updated.blocks[0].text_hash.is_empty());
}
