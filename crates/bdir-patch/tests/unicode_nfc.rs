use bdir_patch::{
    EditPacketV1,
    OpType,
    PatchOpV1,
    PatchV1,
    apply_patch_against_edit_packet,
    validate_patch_against_edit_packet,
};
use bdir_core::hash::{hash_canon_hex, hash_hex};

fn make_packet_single_block(text: &str) -> EditPacketV1 {
    let algo = "sha256";

    let block_id = "p1".to_string();
    let kind_code = 2u16;
    let text_hash = hash_canon_hex(algo, text).expect("sha256 supported");

    let b = vec![(block_id.clone(), kind_code, text_hash.clone(), text.to_string())];

    // Page hash payload: "{block_id}\t{kind_code}\t{text_hash}\n"
    let mut payload = String::new();
    payload.push_str(&block_id);
    payload.push('\t');
    payload.push_str(&kind_code.to_string());
    payload.push('\t');
    payload.push_str(&text_hash);
    payload.push('\n');

    let h = hash_hex(algo, &payload).expect("sha256 supported");

    EditPacketV1 {
        v: 1,
        tid: Some("unicode-nfc-test".to_string()),
        h,
        ha: algo.to_string(),
        b,
    }
}

#[test]
fn validate_and_apply_respects_unicode_nfc_normalization() {
    // Decomposed e + combining acute accent.
    let decomposed = "Cafe\u{0301} au lait";
    let packet = make_packet_single_block(decomposed);

    // Composed 'é' in the patch operation string.
    let composed = "Café au lait";

    let patch = PatchV1 {
        v: 1,
        h: Some(packet.h.clone()),
        ha: Some(packet.ha.clone()),
        ops: vec![PatchOpV1 {
            op: OpType::Replace,
            block_id: "p1".to_string(),
            before: Some(composed.to_string()),
            after: Some("Cafe au lait".to_string()),
            occurrence: None,
            // insert_after fields
            new_block_id: None,
            kind_code: None,
            text: None,
            // suggest fields
            message: None,
            severity: None,
        }],
    };

    // Should validate even though the underlying text uses a different but equivalent Unicode sequence.
    validate_patch_against_edit_packet(&packet, &patch).expect("patch should validate");

    let out = apply_patch_against_edit_packet(&packet, &patch).expect("patch should apply");

    assert_eq!(out.b[0].3, "Cafe au lait");
}
