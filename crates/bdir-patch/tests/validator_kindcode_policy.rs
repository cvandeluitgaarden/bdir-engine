use bdir_core::model::{Block, Document};

use bdir_patch::schema::{OpType, PatchOpV1, PatchV1};
use bdir_patch::validate::{KindCodePolicy, ValidateOptions, validate_patch_with_diagnostics};
use bdir_patch::DiagnosticCode;

fn make_doc() -> Document {
    let mut doc = Document {
        page_hash: "h".to_string(),
        hash_algorithm: "xxh64".to_string(),
        blocks: vec![
            Block {
                id: "core".to_string(),
                kind_code: 2,
                text_hash: "".to_string(),
                text: "Hello world".to_string(),
            },
            Block {
                id: "boiler".to_string(),
                kind_code: 25,
                text_hash: "".to_string(),
                text: "Cookie banner".to_string(),
            },
        ],
    };
    doc.recompute_hashes();
    doc
}

#[test]
fn strict_kindcode_blocks_mutation_outside_default_allow_range() {
    let doc = make_doc();
    let patch = PatchV1 {
        v: 1,
        h: Some(doc.page_hash.clone()),
        ha: Some(doc.hash_algorithm.clone()),
        ops: vec![PatchOpV1 {
            op: OpType::Replace,
            block_id: "boiler".to_string(),
            before: Some("Cookie".to_string()),
            after: Some("Consent".to_string()),
            occurrence: None,
            new_block_id: None,
            kind_code: None,
            text: None,
            message: None,
            severity: None,
        }],
    };

    let mut opts = ValidateOptions::default();
    opts.strict_kind_code = true;

    let err = validate_patch_with_diagnostics(&doc, &patch, opts).unwrap_err();
    assert_eq!(err.diagnostics[0].code, DiagnosticCode::KindCodeDisallowed);
}

#[test]
fn strict_kindcode_allows_suggest_any_by_default() {
    let doc = make_doc();
    let patch = PatchV1 {
        v: 1,
        h: Some(doc.page_hash.clone()),
        ha: Some(doc.hash_algorithm.clone()),
        ops: vec![PatchOpV1 {
            op: OpType::Suggest,
            block_id: "boiler".to_string(),
            before: None,
            after: None,
            occurrence: None,
            new_block_id: None,
            kind_code: None,
            text: None,
            message: Some("Consider minimizing this banner.".to_string()),
            severity: None,
        }],
    };

    let mut opts = ValidateOptions::default();
    opts.strict_kind_code = true;

    validate_patch_with_diagnostics(&doc, &patch, opts).unwrap();
}

#[test]
fn strict_kindcode_allows_custom_ranges() {
    let doc = make_doc();
    let patch = PatchV1 {
        v: 1,
        h: Some(doc.page_hash.clone()),
        ha: Some(doc.hash_algorithm.clone()),
        ops: vec![PatchOpV1 {
            op: OpType::Delete,
            block_id: "boiler".to_string(),
            before: Some("Cookie banner".to_string()),
            after: None,
            occurrence: Some(bdir_patch::schema::Occurrence::Legacy(
                bdir_patch::schema::DeleteOccurrence::First,
            )),
            new_block_id: None,
            kind_code: None,
            text: None,
            message: None,
            severity: None,
        }],
    };

    let mut opts = ValidateOptions::default();
    opts.strict_kind_code = true;
    opts.kind_code_policy = KindCodePolicy {
        allow_ranges: vec![(20, 39)],
        allow_suggest_any: true,
    };

    validate_patch_with_diagnostics(&doc, &patch, opts).unwrap();
}
