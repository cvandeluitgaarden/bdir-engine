mod util;

use bdir_core::model::Document;
use bdir_patch::{validate_patch, PatchV1};

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

    validate_patch(&doc, &patch).expect("valid patch should pass");
}

#[test]
fn unknown_block_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.unknown_block.json");

    let err = validate_patch(&doc, &patch).unwrap_err();
    assert_eq!(
        err,
        "ops[0] references unknown block_id 'does_not_exist'"
    );
}

#[test]
fn before_not_found_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.before_not_found.json");

    let err = validate_patch(&doc, &patch).unwrap_err();
    assert_eq!(
        err,
        "ops[0] (delete) before substring not found in block 'p1'"
    );
}

#[test]
fn before_too_short_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.before_too_short.json");

    let err = validate_patch(&doc, &patch).unwrap_err();
    assert_eq!(
        err,
        "ops[0] before is too short (<8 chars); likely ambiguous"
    );
}

#[test]
fn unsupported_version_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.unsupported_version.json");

    let err = validate_patch(&doc, &patch).unwrap_err();
    assert_eq!(err, "unsupported patch version 2");
}

#[test]
fn replace_missing_after_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.replace_missing_after.json");

    let err = validate_patch(&doc, &patch).unwrap_err();
    assert_eq!(err, "ops[0] (replace) missing after");
}

#[test]
fn suggest_empty_message_fails_with_stable_message() {
    let doc = load_doc();
    let patch = load_patch("patch.suggest_empty_message.json");

    let err = validate_patch(&doc, &patch).unwrap_err();
    assert_eq!(err, "ops[0] (suggest) message is empty");
}

