use std::collections::HashMap;

use bdir_io::{canonical_json, hashing};

#[test]
fn canonical_json_sorts_object_keys() {
    let mut m = HashMap::new();
    m.insert("b", 2);
    m.insert("a", 1);

    let s = canonical_json::to_canonical_json_string(&m).expect("canonical json");
    assert_eq!(s, "{\"a\":1,\"b\":2}");
}

#[test]
fn canonical_hash_stable() {
    let v = serde_json::json!({"z": 9, "a": 1});
    let h1 = hashing::sha256_canonical_json(&v).expect("hash1");
    let h2 = hashing::sha256_canonical_json(&v).expect("hash2");
    assert_eq!(h1, h2);
}

#[test]
fn canonical_hash_diff_on_value_change() {
    let v1 = serde_json::json!({"a": 1});
    let v2 = serde_json::json!({"a": 2});
    let h1 = hashing::sha256_canonical_json(&v1).expect("hash1");
    let h2 = hashing::sha256_canonical_json(&v2).expect("hash2");
    assert_ne!(h1, h2);
}

#[test]
fn canonical_preserves_array_order() {
    let a1 = serde_json::json!([1,2,3]);
    let a2 = serde_json::json!([3,2,1]);
    let h1 = hashing::sha256_canonical_json(&a1).expect("hash1");
    let h2 = hashing::sha256_canonical_json(&a2).expect("hash2");
    assert_ne!(h1, h2);
}
