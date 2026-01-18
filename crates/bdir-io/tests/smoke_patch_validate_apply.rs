use bdir_io::{editpacket, patch};

fn read_json(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn validate_and_apply_golden_examples() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples");

    let packet_s = read_json(&base.join("edit-packet.json"));
    let patch_s = read_json(&base.join("patch.valid.json"));

    let packet: editpacket::EditPacketV1 = serde_json::from_str(&packet_s).expect("edit-packet.json must parse");
    let patch: patch::PatchV1 = serde_json::from_str(&patch_s).expect("patch.valid.json must parse");

    patch::validate_patch_against_edit_packet(&packet, &patch).expect("patch should validate");
    let updated = patch::apply_patch_against_edit_packet(&packet, &patch).expect("patch should apply");

    // Ensure the change is present and other blocks remain.
    let p1 = updated.b.iter().find(|t| t.0 == "p1").expect("p1 block exists");
    assert!(p1.3.contains(": the"), "expected applied text in p1: {}", p1.3);
    assert_eq!(updated.b.len(), 3);
}
