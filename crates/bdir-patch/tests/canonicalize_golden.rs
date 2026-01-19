//! ⚠️ GOLDEN TESTS – DETERMINISTIC ORDERING CONTRACT ⚠️
//!
//! These tests freeze the canonical patch operation ordering.
//!
//! If this test fails, it likely means operation ordering changed.
//! That may be fine, but it MUST be intentional and reviewed, because it
//! affects caching, diff/review noise, and any hashing/signing schemes.

use bdir_patch::canonicalize_patch_ops;
use bdir_patch::schema::PatchV1;

mod util;

fn normalize_for_golden(s: &str) -> String {
    // Make comparison platform-independent:
    // - Normalize Windows CRLF to LF
    // - Trim trailing whitespace/newlines
    s.replace("\r\n", "\n").trim().to_string()
}

#[test]
fn golden_canonicalize_patch_ops_order_is_stable() {
    let unordered_json = util::read_fixture("canonicalize_unordered.patch.json");
    let expected_json = util::read_fixture("canonicalize_expected.patch.json");

    let mut patch: PatchV1 = serde_json::from_str(&unordered_json).unwrap();
    canonicalize_patch_ops(&mut patch);

    let got = serde_json::to_string_pretty(&patch).unwrap();

    assert_eq!(normalize_for_golden(&got), normalize_for_golden(&expected_json));
}
