use serde::{Deserialize, Serialize};

/// Minimal shape needed for validation (matches your ultra-min edit packet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPacketV1 {
    pub v: u8,
    pub h: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ha: Option<String>,
    pub b: Vec<BlockTupleV1>,
}

/// Block tuple: [blockId, kindCode, textHash, text]
pub type BlockTupleV1 = (String, u16, String, String);
