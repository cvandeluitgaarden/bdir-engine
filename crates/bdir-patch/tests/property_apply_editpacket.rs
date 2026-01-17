
use proptest::prelude::*;
use serde_json::json;

use bdir_editpacket::EditPacketV1;
use bdir_patch::schema::PatchV1;
use bdir_patch::apply::apply_patch_against_edit_packet;

fn packet_with_text(text: String) -> EditPacketV1 {
    serde_json::from_value(json!({
        "v": 1,
        "h": "pagehash123",
        "ha": "xxh64",
        "b": [["p1", 2, "h", text]]
    })).unwrap()
}

proptest! {
    #[test]
    fn before_not_found_must_fail(text in ".{0,80}", needle in "[a-zA-Z]{8,12}", after in "[a-zA-Z]{0,12}") {
        prop_assume!(!text.contains(&needle));

        let packet = packet_with_text(text);
        let patch: PatchV1 = serde_json::from_value(json!({
            "v": 1,
            "ops": [{ "op": "replace", "block_id": "p1", "before": needle, "after": after }]
        })).unwrap();

        prop_assert!(apply_patch_against_edit_packet(&packet, &patch).is_err());
    }
}
