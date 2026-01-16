use bdir_core::model::Document;
use bdir_editpacket::{convert::from_document, serialize};

fn read_example_document_json() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("document.json");

    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read examples/document.json at {}: {e}",
            path.display()
        )
    })
}

#[test]
fn golden_edit_packet_pretty_json() {
    let json = read_example_document_json();
    let mut doc: Document = serde_json::from_str(&json).expect("document.json must parse");

    doc.recompute_hashes();
    let packet = from_document(&doc, None);

    let pretty = serialize::to_pretty_json(&packet).expect("pretty json");

    let expected = r#"{
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
}"#;

    assert_eq!(pretty, expected);
}

#[test]
fn golden_edit_packet_minified_json() {
    let json = read_example_document_json();
    let mut doc: Document = serde_json::from_str(&json).expect("document.json must parse");

    doc.recompute_hashes();
    let packet = from_document(&doc, None);

    let min = serialize::to_minified_json(&packet).expect("minified json");

    let expected = r#"{"v":1,"h":"4a0d9b1ad0795617","ha":"xxh64","b":[["t1",0,"2d85646dba5758f4","Example Page Title"],["p1",2,"a3c9cb84972dd67e","This is an example paragraph with a typo teh."],["b1",20,"7a6ea7f684209672","Home > Section > Page"]]}"#;

    assert_eq!(min, expected);
}
