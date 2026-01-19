use anyhow::Result;
use jsonschema::Validator;
use once_cell::sync::Lazy;
use serde_json::Value;
use bdir_io::prelude::*;


static EDIT_PACKET_SCHEMA: Lazy<Result<Validator, String>> = Lazy::new(|| {
    let schema_json: Value = serde_json::from_str(include_str!(
        "../../../spec/schemas/edit-packet.v1.schema.json"
    ))
    .map_err(|e| format!("invalid edit-packet schema JSON: {e}"))?;

    Validator::new(&schema_json)
        .map_err(|e| format!("compile edit-packet schema: {e}"))
});

static PATCH_SCHEMA: Lazy<Result<Validator, String>> = Lazy::new(|| {
    let schema_json: Value = serde_json::from_str(include_str!(
        "../../../spec/schemas/patch.v1.schema.json"
    ))
    .map_err(|e| format!("invalid patch schema JSON: {e}"))?;

    Validator::new(&schema_json)
        .map_err(|e| format!("compile patch schema: {e}"))
});

fn edit_packet_schema() -> &'static Validator {
    EDIT_PACKET_SCHEMA.as_ref().unwrap()
}

fn patch_schema() -> &'static Validator {
    PATCH_SCHEMA.as_ref().unwrap()
}

fn assert_valid(schema: &Validator, instance: &Value) {
    let mut errors = schema.iter_errors(instance).peekable();
    if errors.peek().is_some() {
        let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!("schema validation failed:\n{}", msgs.join("\n"));
    }
}

#[test]
fn examples_conform_to_json_schemas() -> Result<()> {
    let edit_packet: Value = serde_json::from_str(include_str!(
        "../../../examples/edit-packet.json"
    ))?;
    let patch: Value = serde_json::from_str(include_str!(
        "../../../examples/patch.valid.json"
    ))?;

    assert_valid(edit_packet_schema(), &edit_packet);
    assert_valid(patch_schema(), &patch);

    Ok(())
}

#[test]
fn current_wire_types_conform_to_json_schemas() -> Result<()> {
    // Minimal Document -> Edit Packet
    let mut doc = Document {
        page_hash: String::new(),
        hash_algorithm: "xxh64".to_string(),
        blocks: vec![Block {
            id: "p1".to_string(),
            kind_code: 2,
            text_hash: String::new(),
            text: "Hello world".to_string(),
        }],
    };
    doc.recompute_hashes();

    let packet: EditPacketV1 = bdir_io::editpacket::from_document(&doc, None);
    assert_eq!(packet.v, bdir_io::version::EDIT_PACKET_V);
    assert_eq!(packet.ha, "xxh64");

    let packet_json: Value = serde_json::to_value(&packet)?;

    assert_valid(edit_packet_schema(), &packet_json);

    // Patch
    let patch = PatchV1 {
        v: bdir_io::version::PATCH_V,
        h: None,
        ops: vec![PatchOpV1 {
            op: OpType::Suggest,
            occurrence: None,
            block_id: "p1".to_string(),
            before: None,
            after: None,
            content: None,
            message: Some("Looks good".to_string()),
        }],
    };

    let patch_json: Value = serde_json::to_value(&patch)?;
    assert_valid(patch_schema(), &patch_json);

    Ok(())
}

#[test]
fn edit_packet_ha_is_optional_and_defaults_to_sha256() -> Result<()> {
    // RFC-0001: if `ha` is omitted, receivers MUST treat it as "sha256".
    let packet_json: Value = serde_json::json!({
        "v": 1,
        "h": "deadbeef",
        "b": [["p1", 2, "cafebabe", "Hello world"]]
    });

    // JSON Schema must accept omitted `ha`.
    assert_valid(edit_packet_schema(), &packet_json);

    // Wire type must default `ha` when deserializing.
    let packet: EditPacketV1 = serde_json::from_value(packet_json)?;
    assert_eq!(packet.ha, "sha256");

    Ok(())
}
