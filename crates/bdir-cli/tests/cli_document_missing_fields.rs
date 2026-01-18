use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;

#[test]
fn missing_required_top_level_fields_are_actionable() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("bdir_missing_fields_{pid}_{nanos}.json"));

    // hash_algorithm is required, page_hash is optional.
    let doc_json = r#"{
      "page_hash": "x",
      "blocks": [
        {"id":"t1","kind_code":0,"text_hash":"x","text":"Title"}
      ]
    }"#;
    fs::write(&path, doc_json).unwrap();

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", path.to_str().unwrap()]);

    cmd.assert()
        .failure()
        .stderr(contains("missing required top-level field(s): hash_algorithm"))
        .stderr(contains("Required top-level fields: hash_algorithm, blocks"));

    let _ = fs::remove_file(&path);
}
