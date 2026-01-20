mod util;

use bdir_patch::{
    validate_patch_against_edit_packet_with_options,
    EditPacketV1,
    PatchV1,
    ValidateOptions,
};

fn load_edit_packet() -> EditPacketV1 {
    let path = util::workspace_root().join("examples").join("edit-packet.json");
    let json = std::fs::read_to_string(&path).expect("read examples/edit-packet.json");
    serde_json::from_str(&json).expect("edit packet must parse")
}

fn load_patch_fixture(name: &str) -> PatchV1 {
    let json = util::read_fixture(name);
    serde_json::from_str(&json).expect("patch fixture must parse")
}

#[test]
fn strict_mode_rejects_patch_missing_h_even_when_packet_is_available() {
    let packet = load_edit_packet();
    let mut patch = load_patch_fixture("patch.valid.json");
    patch.h = None;
    patch.ha = None;

    let opts = ValidateOptions {
        strict_page_hash_binding: true,
        ..ValidateOptions::default()
    };

    let err = validate_patch_against_edit_packet_with_options(&packet, &patch, opts).unwrap_err();
    assert_eq!(
        err,
        "patch is missing required page hash binding (strict): include patch.h and patch.ha"
    );
}

#[test]
fn strict_mode_rejects_patch_missing_ha_when_h_is_present() {
    let packet = load_edit_packet();
    let mut patch = load_patch_fixture("patch.valid.json");
    patch.ha = None;

    let opts = ValidateOptions {
        strict_page_hash_binding: true,
        ..ValidateOptions::default()
    };

    let err = validate_patch_against_edit_packet_with_options(&packet, &patch, opts).unwrap_err();
    assert_eq!(
        err,
        "patch is missing required hash algorithm binding (strict): include patch.ha"
    );
}
