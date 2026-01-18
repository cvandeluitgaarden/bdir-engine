use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

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
fn validate_patch_ok_exits_0_and_prints_ok() {
    let packet = edit_packet_path();
    let patch = patch_fixture_path("patch.valid.json");

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["validate-patch", packet.to_str().unwrap(), patch.to_str().unwrap()]);

    cmd.assert().success().code(0).stdout("OK\n");
}

#[test]
fn validate_patch_invalid_exits_2_and_prints_error_to_stderr() {
    let packet = edit_packet_path();
    let patch = patch_fixture_path("patch.before_too_short.json");

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["validate-patch", packet.to_str().unwrap(), patch.to_str().unwrap()]);

    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("before is too short"));
}

#[test]
fn validate_patch_invalid_can_emit_structured_diagnostics_json() {
    let packet = edit_packet_path();
    let patch = patch_fixture_path("patch.before_too_short.json");

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args([
        "validate-patch",
        packet.to_str().unwrap(),
        patch.to_str().unwrap(),
        "--diagnostics-json",
    ]);

    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("\"diagnostics\""))
        .stderr(predicate::str::contains("before_too_short"));
}

#[test]
fn validate_patch_short_before_can_be_allowed_with_flag() {
    let packet = edit_packet_path();
    let patch = patch_fixture_path("patch.before_too_short.json");

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args([
        "validate-patch",
        packet.to_str().unwrap(),
        patch.to_str().unwrap(),
        "--min-before-len",
        "4",
    ]);

    cmd.assert().success().code(0).stdout("OK\n");
}

#[test]
fn validate_patch_schema_invalid_exits_1() {
    let packet = edit_packet_path();
    let patch = patch_fixture_path("patch.extra_field.json");

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["validate-patch", packet.to_str().unwrap(), patch.to_str().unwrap()]);

    cmd.assert().failure().code(1);
}
