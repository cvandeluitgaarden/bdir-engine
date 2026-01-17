
use serde_json::json;

use bdir_editpacket::EditPacketV1;
use bdir_patch::schema::PatchV1;
use bdir_patch::apply::apply_patch_against_edit_packet;

struct Case {
    id: &'static str,
    should_pass: bool,
    patch: PatchV1,
}

fn baseline_packet() -> EditPacketV1 {
    serde_json::from_value(json!({
        "v": 1,
        "h": "pagehash123",
        "ha": "xxh64",
        "b": [
            ["p1", 2, "h", "This is teh first paragraph."]
        ]
    })).unwrap()
}

#[test]
fn conformance_matrix() {
    let packet = baseline_packet();

    let cases = vec![
        Case {
            id: "G1",
            should_pass: true,
            patch: serde_json::from_value(json!({
                "v": 1,
                "ops": [{ "op": "replace", "blockId": "p1", "before": "teh first", "after": "the first" }]
            })).unwrap(),
        },
        Case {
            id: "R1",
            should_pass: false,
            patch: serde_json::from_value(json!({
                "v": 1,
                "ops": [{ "op": "replace", "blockId": "nope", "before": "teh first", "after": "the first" }]
            })).unwrap(),
        },
        Case {
            id: "R2",
            should_pass: false,
            patch: serde_json::from_value(json!({
                "v": 1,
                "ops": [{ "op": "replace", "blockId": "p1", "before": "short", "after": "the first" }]
            })).unwrap(),
        },
    ];

    let mut passed = 0usize;
    let total = cases.len();

    for c in cases {
        let ok = apply_patch_against_edit_packet(&packet, &c.patch).is_ok();
        if ok == c.should_pass {
            passed += 1;
        } else {
            panic!("Conformance failure: {}", c.id);
        }
    }

    eprintln!("BDIR patch apply conformance: {passed}/{total}");
    eprintln!("badge: bdir-apply-conformance={passed}-{total}");
}
