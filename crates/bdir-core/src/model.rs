use serde::{Deserialize, Serialize};

/// A stable identifier for a block.
pub type BlockId = String;

/// A single semantic block in a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub kind_code: u16,
    pub text_hash: String,
    pub text: String,
}

/// A document as ordered blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Page-level content hash.
    pub page_hash: String,
    /// Hash algorithm for `text_hash` values (e.g., xxh64, sha256).
    pub hash_algorithm: String,
    pub blocks: Vec<Block>,
}
