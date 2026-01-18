mod common;
use bdir_core::model::Document;
use common::{util_fs, util_hash}; 

#[test]
fn golden_hashes_example_document_v1() {
    let json = util_fs::read_example_document_json();
    let mut doc: Document = serde_json::from_str(&json).expect("document.json must parse");

    doc.recompute_hashes();

    assert!(util_hash::is_hex16(&doc.page_hash), "page_hash should be 16-char lowercase hex");
    for b in &doc.blocks {
        assert!(util_hash::is_hex16(&b.text_hash), "block text_hash should be 16-char lowercase hex");
    }


    // ---- GOLDEN ASSERTS ----
    assert_eq!(doc.hash_algorithm, "xxh64");
    assert_eq!(doc.page_hash, "4a0d9b1ad0795617");

    assert_eq!(doc.blocks.len(), 3);

    assert_eq!(doc.blocks[0].id, "t1");
    assert_eq!(doc.blocks[0].kind_code, 0);
    assert_eq!(doc.blocks[0].text_hash, "2d85646dba5758f4");
    assert_eq!(doc.blocks[0].text, "Example Page Title");

    assert_eq!(doc.blocks[1].id, "p1");
    assert_eq!(doc.blocks[1].kind_code, 2);
    assert_eq!(doc.blocks[1].text_hash, "a3c9cb84972dd67e");
    assert_eq!(doc.blocks[1].text, "This is an example paragraph with a typo teh.");

    assert_eq!(doc.blocks[2].id, "b1");
    assert_eq!(doc.blocks[2].kind_code, 20);
    assert_eq!(doc.blocks[2].text_hash, "7a6ea7f684209672");
    assert_eq!(doc.blocks[2].text, "Home > Section > Page");
}
