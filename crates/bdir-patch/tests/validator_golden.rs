mod util;

use bdir_core::model::Document;
use bdir_patch::{
    DiagnosticCode,
    ValidateOptions,
    validate_patch_with_options,
    validate_patch_with_diagnostics,
    PatchV1,
};

fn load_doc() -> Document {
    let json = util::read_example_document_json();
    let mut doc: Document = serde_json::from_str(&json).expect("document.json must parse");
    doc.recompute_hashes();
    doc
}

fn load_patch(fixture: &str) -> PatchV1 {
    let json = util::read_fixture(fixture);
    serde_json::from_str(&json).expect("patch fixture must parse")
}

#[test]
fn valid_patch_passes() {
    let doc = load_doc();
    let patch = load_patch("patch.valid.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    validate_patch_with_options(&doc, &patch, opts).expect("valid patch should pass");
}

#[test]
fn unknown_block_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.unknown_block.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert_eq!(
        err,
        "ops[0] references unknown block_id 'does_not_exist'"
    );
}

#[test]
fn before_not_found_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.before_not_found.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert_eq!(
        err,
        "ops[0] (delete) before substring not found in block 'p1'"
    );
}

#[test]
fn delete_missing_occurrence_is_allowed_when_unambiguous() {
    let doc = load_doc();
    let patch = load_patch("patch.delete_missing_occurrence.json");

    // RFC-0001 v1.0.2: `occurrence` is optional and only required when `before`
    // matches multiple times within the block.
    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    validate_patch_with_options(&doc, &patch, opts).expect("missing occurrence should be accepted when unambiguous");
}

#[test]
fn delete_without_occurrence_is_rejected_when_ambiguous() {
    let mut doc = load_doc();
    // Make the match ambiguous.
    doc.blocks.iter_mut().find(|b| b.id == "p1").unwrap().text = "DELETE_ME DELETE_ME".to_string();
    doc.recompute_hashes();

    let patch: PatchV1 = serde_json::from_value(serde_json::json!({
        "v": 1,
        "ops": [
            { "op": "delete", "block_id": "p1", "before": "DELETE_ME" }
        ]
    })).unwrap();

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert!(err.contains("ambiguous"));
}

#[test]
fn before_too_short_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.before_too_short.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert_eq!(
        err,
        "ops[0] before is too short (<8 chars); likely ambiguous"
    );
}

#[test]
fn diagnostics_surface_code_path_and_message() {
    let doc = load_doc();
    let patch = load_patch("patch.before_too_short.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_diagnostics(&doc, &patch, opts)
        .expect_err("expected validation to fail");
    let diag = err.diagnostics.first().expect("at least one diagnostic");

    assert_eq!(diag.code, DiagnosticCode::BeforeTooShort);
    assert_eq!(diag.path.as_deref(), Some("ops[0].before"));
    assert!(diag.message.contains("before is too short"));
}

#[test]
fn before_too_short_can_be_enabled_via_options() {
    let doc = load_doc();
    let patch = load_patch("patch.before_too_short.json");

    // The fixture uses a short `before`. By default it is rejected (see test above),
    // but it can be allowed by explicitly lowering the guard.
    validate_patch_with_options(
        &doc,
        &patch,
        ValidateOptions {
            min_before_len: 4,
            expected_page_hash: Some(doc.page_hash.clone()),
            ..ValidateOptions::default()
        },
    )
        .expect("short before should be accepted when configured");
}

#[test]
fn unsupported_version_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.unsupported_version.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert_eq!(err, "unsupported patch version 2");
}

#[test]
fn replace_missing_after_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.replace_missing_after.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert_eq!(err, "ops[0] (replace) missing after");
}

#[test]
fn suggest_empty_message_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.suggest_empty_message.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert_eq!(err, "ops[0] (suggest) message is empty");
}

#[test]
fn suggest_with_before_is_rejected() {
    let doc = load_doc();
    let patch = load_patch("patch.suggest_with_before.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert_eq!(
        err,
        "ops[0] (suggest) unexpected before (suggest must not include before/after)"
    );
}

#[test]
fn page_hash_mismatch_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.page_hash_mismatch.json");

    let opts = ValidateOptions { expected_page_hash: Some(doc.page_hash.clone()), ..ValidateOptions::default() };
    let err = validate_patch_with_options(&doc, &patch, opts).unwrap_err();
    assert_eq!(
        err,
        format!(
            "patch page hash mismatch (patch.h='__MISMATCH__' differs from expected_page_hash='{}')",
            doc.page_hash
        )
    );
}


#[test]
fn missing_page_hash_binding_is_rejected_by_default() {
    let doc = load_doc();
    let mut patch = load_patch("patch.valid.json");
    patch.h = None;

    let err = validate_patch_with_options(&doc, &patch, ValidateOptions::default()).unwrap_err();
    assert_eq!(
        err,
        "patch is missing required page hash binding: include patch.h or provide expected_page_hash"
    );
}
