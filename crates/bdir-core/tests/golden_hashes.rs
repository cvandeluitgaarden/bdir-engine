use bdir_core::model::Document;

fn is_hex16(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[test]
fn golden_hashes_example_document_v1() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join("..")
    .join("examples")
    .join("document.json");

    // Load the example document JSON from the repo root.
    let json = std::fs::read_to_string(&path)
    .expect("examples/document.json must exist");

    let mut doc: Document = serde_json::from_str(&json).expect("document.json must parse");

    doc.recompute_hashes();

    // Basic sanity
    assert!(is_hex16(&doc.page_hash), "page_hash should be 16-char lowercase hex");
    assert!(!doc.blocks.is_empty(), "document should have blocks");
    for b in &doc.blocks {
        assert!(is_hex16(&b.text_hash), "block text_hash should be 16-char lowercase hex");
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
