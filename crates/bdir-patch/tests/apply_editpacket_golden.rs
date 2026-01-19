//! ⚠️ GOLDEN TESTS – PROTOCOL CONTRACT ⚠️
//!
//! These tests define the frozen, externally visible semantics of
//! `apply_patch_against_edit_packet`.
//!
//! Any change here MUST be intentional and reviewed as a protocol change.
//! If a test fails, fix the implementation – do not update expectations
//! unless the protocol itself is evolving.

use serde_json::json;

use bdir_editpacket::EditPacketV1;
use bdir_patch::schema::PatchV1;
use bdir_patch::apply::apply_patch_against_edit_packet;

fn baseline_packet() -> EditPacketV1 {
    serde_json::from_value(json!({
        "v": 1,
        "tid": "test-001",
        "h": "pagehash123",
        "ha": "xxh64",
        "b": [
            ["p1", 2, "hash-a", "This is teh first paragraph. This is teh first paragraph."],
            ["p2", 2, "hash-b", "This is the second paragraph."]
        ]
    })).unwrap()
}

#[test]
fn golden_replace_replaces_first_occurrence_only() {
    let packet = baseline_packet();

    // before must be >= 8 chars per validator
    let patch: PatchV1 = serde_json::from_value(json!({
        "v": 1,
        "ops": [
            { "op": "replace", "block_id": "p1", "before": "teh first", "after": "the first" }
        ]
    })).unwrap();

    let patch = bind_patch_to_edit_packet(patch, &packet);
    let out = apply_patch_against_edit_packet(&packet, &patch).unwrap();

    let text = &out.b[0].3;
    // first occurrence replaced, second remains
    assert!(text.contains("the first paragraph."));
    assert!(text.contains("teh first paragraph."));
}

#[test]
fn golden_delete_removes_all_occurrences() {
    let mut packet = baseline_packet();
    packet.b[1].3 = "DELETE_ME DELETE_ME DELETE_ME".to_string();

    let patch: PatchV1 = serde_json::from_value(json!({
        "v": 1,
        "ops": [
            { "op": "delete", "block_id": "p2", "before": "DELETE_ME", "occurrence": "all" }
        ]
    })).unwrap();

    let patch = bind_patch_to_edit_packet(patch, &packet);
    let out = apply_patch_against_edit_packet(&packet, &patch).unwrap();
    assert_eq!(out.b[1].3.trim(), "");
}

#[test]
fn golden_delete_removes_first_occurrence_only() {
    let mut packet = baseline_packet();
    packet.b[1].3 = "DELETE_ME DELETE_ME DELETE_ME".to_string();

    let patch: PatchV1 = serde_json::from_value(json!({
        "v": 1,
        "ops": [
            { "op": "delete", "block_id": "p2", "before": "DELETE_ME", "occurrence": "first" }
        ]
    })).unwrap();

    let patch = bind_patch_to_edit_packet(patch, &packet);
    let out = apply_patch_against_edit_packet(&packet, &patch).unwrap();
    assert_eq!(out.b[1].3.trim(), "DELETE_ME DELETE_ME");
}

#[test]
fn golden_insert_after_inserts_new_block_with_deterministic_id_and_inherited_kind() {
    let packet = baseline_packet();

    let patch: PatchV1 = serde_json::from_value(json!({
        "v": 1,
        "ops": [
            { "op": "insert_after", "block_id": "p1", "content": "Inserted block text." }
        ]
    })).unwrap();

    let patch = bind_patch_to_edit_packet(patch, &packet);
    let out = apply_patch_against_edit_packet(&packet, &patch).unwrap();

    // Inserted immediately after p1
    assert_eq!(out.b[1].0, "p1_ins");
    assert_eq!(out.b[1].1, out.b[0].1); // inherits kindCode
    assert_eq!(out.b[1].3, "Inserted block text.");
}

#[test]
fn golden_suggest_is_non_mutating() {
    let packet = baseline_packet();

    let patch: PatchV1 = serde_json::from_value(json!({
        "v": 1,
        "ops": [
            { "op": "suggest", "block_id": "p2", "message": "Consider simplifying." }
        ]
    })).unwrap();

    let patch = bind_patch_to_edit_packet(patch, &packet);
    let out = apply_patch_against_edit_packet(&packet, &patch).unwrap();
    assert_eq!(out.b[1].3, "This is the second paragraph.");
}

#[test]
fn reject_unknown_block_id() {
    let packet = baseline_packet();

    let patch: PatchV1 = serde_json::from_value(json!({
        "v": 1,
        "ops": [
            { "op": "replace", "block_id": "nope", "before": "teh first", "after": "the first" }
        ]
    })).unwrap();

    let patch = bind_patch_to_edit_packet(patch, &packet);
    let err = apply_patch_against_edit_packet(&packet, &patch).unwrap_err();
    assert!(err.contains("references unknown block_id"));
}

#[test]
fn reject_before_too_short() {
    let packet = baseline_packet();

    let patch: PatchV1 = serde_json::from_value(json!({
        "v": 1,
        "ops": [
            { "op": "replace", "block_id": "p1", "before": "short", "after": "longer" }
        ]
    })).unwrap();

    let patch = bind_patch_to_edit_packet(patch, &packet);
    let err = apply_patch_against_edit_packet(&packet, &patch).unwrap_err();
    assert!(err.contains("before is too short"));
}

fn bind_patch_to_edit_packet(mut patch: PatchV1, ep: &EditPacketV1) -> PatchV1 {
    patch.h = Some(ep.h.clone());
    patch
}