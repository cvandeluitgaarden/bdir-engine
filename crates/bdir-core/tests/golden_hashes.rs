mod util_fs;
mod util_hash;
use bdir_core::model::Document;

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
    // Replace the values below once, then they should never change unintentionally.
    assert_eq!(doc.hash_algorithm, "xxh64");

    // TODO: paste computed page hash
    assert_eq!(doc.page_hash, "4a0d9b1ad0795617");

    // TODO: paste computed block hashes
    assert_eq!(doc.blocks[0].id, "t1");
    assert_eq!(doc.blocks[0].text_hash, "2d85646dba5758f4");

    assert_eq!(doc.blocks[1].id, "p1");
    assert_eq!(doc.blocks[1].text_hash, "a3c9cb84972dd67e");

    assert_eq!(doc.blocks[2].id, "b1");
    assert_eq!(doc.blocks[2].text_hash, "7a6ea7f684209672");
}
