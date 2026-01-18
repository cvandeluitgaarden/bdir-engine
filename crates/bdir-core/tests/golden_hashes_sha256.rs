mod common;
use bdir_core::model::Document;
use common::{util_fs, util_hash};

#[test]
fn golden_hashes_example_document_sha256() {
    let json = util_fs::read_example_document_json();
    let mut doc: Document = serde_json::from_str(&json).expect("document.json must parse");

    // Switch to sha256 and recompute.
    doc.hash_algorithm = "sha256".to_string();
    doc.recompute_hashes();

    assert!(
        util_hash::is_hex64(&doc.page_hash),
        "page_hash should be 64-char lowercase hex"
    );
    for b in &doc.blocks {
        assert!(
            util_hash::is_hex64(&b.text_hash),
            "block text_hash should be 64-char lowercase hex"
        );
    }

    // ---- GOLDEN ASSERTS ----
    assert_eq!(doc.hash_algorithm, "sha256");
    assert_eq!(
        doc.page_hash,
        "ed16af3e8f130bb55274a73f3f0635e37605c21ed3c03f9917d830ab76c64df1"
    );

    assert_eq!(doc.blocks.len(), 3);

    assert_eq!(doc.blocks[0].id, "t1");
    assert_eq!(doc.blocks[0].kind_code, 0);
    assert_eq!(
        doc.blocks[0].text_hash,
        "4946647938d23aabecb1091a35f89256311be8b6a8ad573f8ea035cccb128a97"
    );
    assert_eq!(doc.blocks[0].text, "Example Page Title");

    assert_eq!(doc.blocks[1].id, "p1");
    assert_eq!(doc.blocks[1].kind_code, 2);
    assert_eq!(
        doc.blocks[1].text_hash,
        "7633b0f00cfe8fac4cd37e94337c8133e92897ce663a12cb4f40e72d16157651"
    );
    assert_eq!(doc.blocks[1].text, "This is an example paragraph with a typo teh.");

    assert_eq!(doc.blocks[2].id, "b1");
    assert_eq!(doc.blocks[2].kind_code, 20);
    assert_eq!(
        doc.blocks[2].text_hash,
        "8b8ffa61bb51297a7e2c31ab05313a05feb36e20f58eaf8cb35b159be5d3759e"
    );
    assert_eq!(doc.blocks[2].text, "Home > Section > Page");
}
