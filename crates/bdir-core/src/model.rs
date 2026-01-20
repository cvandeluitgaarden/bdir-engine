use serde::{Deserialize, Serialize};
use crate::hash::{hash_canon_hex, hash_hex};

/// A stable identifier for a block.
pub type BlockId = String;

/// A single semantic block in a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub kind_code: u16,
    #[serde(default)]
    pub text_hash: String,
    pub text: String,
}

/// A document as ordered blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Page-level content hash.
    #[serde(default)]
    pub page_hash: String,
    /// Hash algorithm for `text_hash` values (e.g., xxh64, sha256).
    pub hash_algorithm: String,
    pub blocks: Vec<Block>,
}

impl Document {
    /// Normalize and validate the document's declared `hash_algorithm`.
    ///
    /// Normalization:
    /// - Trims surrounding whitespace
    /// - Lowercases for canonical representation
    ///
    /// Validation:
    /// - Returns an error if the algorithm is empty or unsupported.
    ///
    /// RFC-0001 (v1.0.2) requires receivers to reject unrecognized
    /// `hash_algorithm` values rather than coercing them.
    pub fn normalize_hash_algorithm(&mut self) -> Result<(), String> {
        // NOTE: We must avoid holding a `&str` borrow into `self.hash_algorithm` across assignment.
        let algo = self.hash_algorithm.trim().to_lowercase();
        if algo.is_empty() {
            return Err("hash_algorithm is empty".to_string());
        }
        if hash_hex(&algo, "").is_none() {
            return Err(format!("unsupported hash_algorithm '{algo}'"));
        }
        self.hash_algorithm = algo;
        Ok(())
    }

    /// Recompute block `text_hash` values (from block text) and `page_hash`
    /// deterministically.
    ///
    /// Returns an error if the document's `hash_algorithm` is unsupported.
    pub fn try_recompute_hashes(&mut self) -> Result<(), String> {
        self.normalize_hash_algorithm()?;

        let algo = self.hash_algorithm.clone();
        for b in &mut self.blocks {
            b.text_hash = hash_canon_hex(&algo, &b.text).expect("supported algorithm");
        }

        let mut page_payload = String::new();
        for b in &self.blocks {
            page_payload.push_str(&b.id);
            page_payload.push('\t');
            page_payload.push_str(&b.kind_code.to_string());
            page_payload.push('\t');
            page_payload.push_str(&b.text_hash);
            page_payload.push('\n');
        }

        self.page_hash = hash_hex(&algo, &page_payload).expect("supported algorithm");
        Ok(())
    }

    /// Convenience wrapper that panics on unsupported algorithms.
    ///
    /// This preserves the existing API shape for internal callers/tests that
    /// assume a valid algorithm, while keeping RFC behavior available via
    /// `try_recompute_hashes()`.
    pub fn recompute_hashes(&mut self) {
        self.try_recompute_hashes().expect("supported hash_algorithm");
    }
}
