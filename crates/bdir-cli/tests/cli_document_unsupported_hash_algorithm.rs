use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;

#[test]
fn unsupported_hash_algorithm_is_rejected() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "bdir_unsupported_hash_algorithm_{pid}_{nanos}.json"
    ));

    let doc_json = r#"{
      "hash_algorithm": "md5",
      "blocks": [
        {"id":"t1","kind_code":0,"text_hash":"x","text":"Title"}
      ]
    }"#;
    fs::write(&path, doc_json).unwrap();

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", path.to_str().unwrap()]);

    cmd.assert()
        .failure()
        .stderr(contains("Unsupported hash_algorithm"))
        .stderr(contains("Supported algorithms"));

    let _ = fs::remove_file(&path);
}
