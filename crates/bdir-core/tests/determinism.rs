use bdir_core::model::Document;

#[test]
fn recompute_hashes_is_deterministic() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("document.json");
    
    let json = std::fs::read_to_string(&path)
        .expect("examples/document.json must exist");

    let mut doc1: Document = serde_json::from_str(&json).expect("document.json must parse");
    let mut doc2: Document = serde_json::from_str(&json).expect("document.json must parse");

    doc1.recompute_hashes();
    doc2.recompute_hashes();

    assert_eq!(doc1.hash_algorithm, doc2.hash_algorithm);
    assert_eq!(doc1.page_hash, doc2.page_hash);
    assert_eq!(doc1.blocks.len(), doc2.blocks.len());

    for (a, b) in doc1.blocks.iter().zip(doc2.blocks.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.kind_code, b.kind_code);
        assert_eq!(a.text_hash, b.text_hash);
        assert_eq!(a.text, b.text);
    }
}
