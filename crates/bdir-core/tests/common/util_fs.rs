use std::path::{Path, PathBuf};

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

pub fn read_example_document_json() -> String {
    let path = workspace_root()
        .join("examples")
        .join("document.json");

    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read examples/document.json at {}: {e}", path.display())
    })
}