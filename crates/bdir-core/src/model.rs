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
    /// Recompute block `text_hash` values (from block text) and `page_hash`
    /// deterministically.
    ///
    /// Page hash is computed over ordered lines:
    /// `{blockId}\t{kindCode}\t{textHash}\n`
    pub fn recompute_hashes(&mut self) {
        // Respect the document's declared algorithm. Default to xxh64 if empty/unknown.
        // NOTE: We must avoid holding a `&str` borrow into `self.hash_algorithm` across assignment.
        let mut algo = self.hash_algorithm.trim().to_lowercase();
        if algo.is_empty() || hash_hex(&algo, "").is_none() {
            algo = "xxh64".to_string();
        }
        self.hash_algorithm = algo.clone();

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
    }
}
