use assert_cmd::cargo::cargo_bin_cmd;

use bdir_io::prelude::Document;

fn example_document_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("document.json")
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
fn cli_apply_patch_document_writes_updated_document_json() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let input = example_document_path();
    let patch = patch_fixture_path("patch.valid.json");

    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let out_path = std::env::temp_dir().join(format!("bdir_apply_doc_{pid}_{nanos}.json"));

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args([
        "apply-patch",
        "--doc",
        input.to_str().unwrap(),
        "--patch",
        patch.to_str().unwrap(),
        "--out",
        out_path.to_str().unwrap(),
    ]);

    cmd.assert().success();

    let out_s = fs::read_to_string(&out_path).unwrap();
    let updated: Document = serde_json::from_str(&out_s).unwrap();

    let p1 = updated.blocks.iter().find(|b| b.id == "p1").unwrap();
    assert!(p1.text.contains("example paragraph with a typo: the"));

    let _ = fs::remove_file(&out_path);
}
