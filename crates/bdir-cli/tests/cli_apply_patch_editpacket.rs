use assert_cmd::cargo::cargo_bin_cmd;

use bdir_io::prelude::EditPacketV1;

fn edit_packet_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("edit-packet.json")
}

fn patch_fixture_path(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("bdir-patch")
        .join("tests")
        .join("fixtures")
        .join(file)
}

#[test]
fn cli_apply_patch_edit_packet_stdout_is_updated_edit_packet() {
    let packet = edit_packet_path();
    let patch = patch_fixture_path("patch.valid.json");

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args([
        "apply-patch",
        packet.to_str().unwrap(),
        patch.to_str().unwrap(),
    ]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let out_s = String::from_utf8(output).unwrap();
    let updated: EditPacketV1 = serde_json::from_str(&out_s).unwrap();

    let p1 = updated.b.iter().find(|t| t.0 == "p1").unwrap();
    assert!(p1.3.contains("example paragraph with a typo: the"));
}
