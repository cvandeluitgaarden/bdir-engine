use assert_cmd::cargo::cargo_bin_cmd;
use bdir_io::prelude::EditPacketV1;

fn large_fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("large-document")
        .join("document.json")
}

#[test]
fn cli_edit_packet_large_fixture_succeeds_and_is_large() {
    let input = large_fixture_path();

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["edit-packet", input.to_str().unwrap(), "--min"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let out_s = String::from_utf8(output).unwrap();

    let packet: EditPacketV1 = serde_json::from_str(&out_s).unwrap();

    // This fixture is intended to be "large" enough to stress the CLI and engine.
    // Keep the threshold conservative to avoid flakiness if the fixture evolves.
    assert!(packet.b.len() >= 400, "expected >= 400 blocks, got {}", packet.b.len());

    // A quick sanity check that we include core content and boilerplate.
    assert!(packet.b.iter().any(|t| t.1 <= 19));
    assert!(packet.b.iter().any(|t| t.1 >= 20));
}

#[test]
fn cli_inspect_large_fixture_emits_expected_header() {
    let input = large_fixture_path();

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", input.to_str().unwrap()]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let out_s = String::from_utf8(output).unwrap();

    // Non-interactive output is tab-separated with a stable header.
    assert!(out_s.lines().next().unwrap().starts_with("blockId\tkindCode\timportance\ttextHash\tpreview"));

    // Should include many lines (header + blocks). This also catches truncation/panic bugs.
    assert!(out_s.lines().count() >= 401);
}
