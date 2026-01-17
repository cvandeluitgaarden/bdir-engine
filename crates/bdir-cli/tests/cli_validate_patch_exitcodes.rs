use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn root_examples_path(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join(file)
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
    let doc = root_examples_path("document.json");
    let patch = patch_fixture_path("patch.valid.json");

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args([
        "validate-patch",
        doc.to_str().unwrap(),
        patch.to_str().unwrap(),
    ]);

    cmd.assert()
        .success()
        .code(0)
        .stdout("OK\n");
}

#[test]
fn validate_patch_invalid_exits_2_and_prints_error_to_stderr() {
    let doc = root_examples_path("document.json");
    let patch = patch_fixture_path("patch.before_too_short.json");

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args([
        "validate-patch",
        doc.to_str().unwrap(),
        patch.to_str().unwrap(),
    ]);

    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("before is too short"));
}
