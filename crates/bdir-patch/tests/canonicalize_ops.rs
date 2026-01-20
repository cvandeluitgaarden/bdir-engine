use bdir_patch::{
    canonicalize_patch_ops,
    canonicalize_patch_ops_against_edit_packet,
    schema::{DeleteOccurrence, Occurrence, OpType, PatchOpV1, PatchV1},
    EditPacketV1,
};

fn mk_patch(ops: Vec<PatchOpV1>) -> PatchV1 {
    PatchV1 { v: 1, h: None, ops }
}

fn op(op: OpType, block_id: &str) -> PatchOpV1 {
    PatchOpV1 {
        op,
        block_id: block_id.to_string(),
        before: None,
        after: None,
        occurrence: None,
        content: None,
        message: None,
    }
}

#[test]
fn canonicalize_falls_back_to_blockid_and_op_rank() {
    let mut patch = mk_patch(vec![
        op(OpType::Suggest, "b"),
        op(OpType::Delete, "a"),
        op(OpType::Replace, "a"),
        op(OpType::InsertAfter, "a"),
        op(OpType::Delete, "b"),
    ]);

    canonicalize_patch_ops(&mut patch);

    let got: Vec<(OpType, String)> = patch
        .ops
        .iter()
        .map(|o| (o.op, o.block_id.clone()))
        .collect();

    // block_id asc, then delete, replace, insert_after, suggest
    let expect = vec![
        (OpType::Delete, "a".to_string()),
        (OpType::Replace, "a".to_string()),
        (OpType::InsertAfter, "a".to_string()),
        (OpType::Delete, "b".to_string()),
        (OpType::Suggest, "b".to_string()),
    ];

    assert_eq!(got, expect);
}

#[test]
fn canonicalize_prefers_edit_packet_block_order() {
    let packet = EditPacketV1 {
        v: 1,
        tid: None,
        h: "pagehash".to_string(),
        ha: "xxh64".to_string(),
        b: vec![
            ("p2".to_string(), 2, "h2".to_string(), "two".to_string()),
            ("p1".to_string(), 2, "h1".to_string(), "one".to_string()),
        ],
    };

    let mut patch = mk_patch(vec![
        op(OpType::Replace, "p1"),
        op(OpType::Delete, "p2"),
    ]);

    canonicalize_patch_ops_against_edit_packet(&packet, &mut patch);

    assert_eq!(patch.ops[0].block_id, "p2");
    assert_eq!(patch.ops[1].block_id, "p1");
}

#[test]
fn canonicalize_orders_delete_occurrence_first_then_all() {
    let mut o1 = op(OpType::Delete, "p1");
    o1.before = Some("x".to_string());
    o1.occurrence = Some(Occurrence::Legacy(DeleteOccurrence::All));

    let mut o2 = op(OpType::Delete, "p1");
    o2.before = Some("x".to_string());
    o2.occurrence = Some(Occurrence::Legacy(DeleteOccurrence::First));

    let mut patch = mk_patch(vec![o1, o2]);
    canonicalize_patch_ops(&mut patch);

    assert_eq!(patch.ops[0].occurrence, Some(Occurrence::Legacy(DeleteOccurrence::First)));
    assert_eq!(patch.ops[1].occurrence, Some(Occurrence::Legacy(DeleteOccurrence::All)));
}
