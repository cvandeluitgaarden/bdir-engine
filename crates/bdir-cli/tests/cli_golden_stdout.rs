use assert_cmd::cargo::cargo_bin_cmd;

fn example_document_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("document.json")
}

#[test]
fn cli_edit_packet_pretty_stdout_golden() {
    let input = example_document_path();

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["edit-packet", input.to_str().unwrap()]);

    cmd.assert()
        .success()
        .stdout(
            r#"{
  "v": 1,
  "h": "4a0d9b1ad0795617",
  "ha": "xxh64",
  "b": [
    [
      "t1",
      0,
      "2d85646dba5758f4",
      "Example Page Title"
    ],
    [
      "p1",
      2,
      "a3c9cb84972dd67e",
      "This is an example paragraph with a typo teh."
    ],
    [
      "b1",
      20,
      "7a6ea7f684209672",
      "Home > Section > Page"
    ]
  ]
}
"#,
        );
}

#[test]
fn cli_edit_packet_minified_stdout_golden() {
    let input = example_document_path();

    let mut cmd = cargo_bin_cmd!("bdir");
    cmd.args(["edit-packet", input.to_str().unwrap(), "--min"]);

    cmd.assert()
        .success()
        // NOTE: println! adds a trailing newline. If you switch to print! in the CLI,
        // remove the trailing "\n" here.
        .stdout(
            r#"{"v":1,"h":"4a0d9b1ad0795617","ha":"xxh64","b":[["t1",0,"2d85646dba5758f4","Example Page Title"],["p1",2,"a3c9cb84972dd67e","This is an example paragraph with a typo teh."],["b1",20,"7a6ea7f684209672","Home > Section > Page"]]}
"#,
        );
}
