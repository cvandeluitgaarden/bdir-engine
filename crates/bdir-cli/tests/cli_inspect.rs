use assert_cmd::cargo::cargo_bin_cmd;

fn example_document_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("document.json")
}

#[test]
fn cli_inspect_stdout_golden() {
    let input = example_document_path();

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", input.to_str().unwrap()]);

    cmd.assert().success().stdout(
        "blockId\tkindCode\timportance\ttextHash\tpreview\n\
t1\t0\tcore\t2d85646dba5758f4\tExample Page Title\n\
p1\t2\tcore\ta3c9cb84972dd67e\tThis is an example paragraph with a typo teh.\n\
b1\t20\tboilerplate\t7a6ea7f684209672\tHome > Section > Page\n",
    );
}

#[test]
fn cli_inspect_filters_work() {
    let input = example_document_path();

    // --kind single
    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", input.to_str().unwrap(), "--kind", "0"]);
    cmd.assert().success().stdout(
        "blockId\tkindCode\timportance\ttextHash\tpreview\n\
t1\t0\tcore\t2d85646dba5758f4\tExample Page Title\n",
    );

    // --kind range
    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", input.to_str().unwrap(), "--kind", "0-2"]);
    cmd.assert().success().stdout(
        "blockId\tkindCode\timportance\ttextHash\tpreview\n\
t1\t0\tcore\t2d85646dba5758f4\tExample Page Title\n\
p1\t2\tcore\ta3c9cb84972dd67e\tThis is an example paragraph with a typo teh.\n",
    );

    // --id exact
    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", input.to_str().unwrap(), "--id", "b1"]);
    cmd.assert().success().stdout(
        "blockId\tkindCode\timportance\ttextHash\tpreview\n\
b1\t20\tboilerplate\t7a6ea7f684209672\tHome > Section > Page\n",
    );

    // --grep substring
    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", input.to_str().unwrap(), "--grep", "typo"]);
    cmd.assert().success().stdout(
        "blockId\tkindCode\timportance\ttextHash\tpreview\n\
p1\t2\tcore\ta3c9cb84972dd67e\tThis is an example paragraph with a typo teh.\n",
    );
}

#[test]
fn cli_inspect_preview_is_bounded() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("bdir_inspect_long_{pid}_{nanos}.json"));

    let long_text = "a".repeat(200);
    let doc_json = format!(
        r#"{{"page_hash":"x","hash_algorithm":"xxh64","blocks":[{{"id":"x1","kind_code":1,"text_hash":"x","text":{:?}}}]}}"#,
        long_text
    );
    fs::write(&path, doc_json).unwrap();

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["inspect", path.to_str().unwrap()]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let out = String::from_utf8(out).unwrap();
    let mut lines = out.lines();
    let _header = lines.next().unwrap();
    let row = lines.next().unwrap();
    let cols: Vec<&str> = row.split('\t').collect();
    assert_eq!(cols.len(), 5);
    let preview = cols[4];

    // 80-char bound, with ellipsis when truncated.
    assert!(preview.chars().count() <= 80);
    assert!(preview.ends_with('…'));

    let _ = fs::remove_file(&path);
}
