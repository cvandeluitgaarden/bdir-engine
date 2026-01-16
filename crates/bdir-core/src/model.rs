use serde::{Deserialize, Serialize};
use crate::hash::{canonicalize_text, xxh64_hex};

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
        // Ensure the document advertises the algorithm actually used.
        // (If you want to respect an existing value, remove this line.)
        self.hash_algorithm = "xxh64".to_string();

        for b in &mut self.blocks {
            let canon = canonicalize_text(&b.text);
            b.text_hash = xxh64_hex(&canon);
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

        self.page_hash = xxh64_hex(&page_payload);
    }
}
