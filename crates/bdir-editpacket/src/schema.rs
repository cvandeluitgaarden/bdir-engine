use serde::{Deserialize, Serialize};

/// Ultra-minimal BDIR Edit Packet v1.
///
/// RFC-0001 wire format:
/// {
///   "v": 1,
///   "tid": "optional",
///   "h": "pageHash",
///   "ha": "xxh64",
///   "b": [["blockId", kindCode, "textHash", "text"]]
/// }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPacketV1 {
    /// Version (const = 1)
    pub v: u8,
    /// Optional trace id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tid: Option<String>,
    /// Page-level hash
    pub h: String,
    /// Hash algorithm (required in RFC-0001)
    pub ha: String,
    /// Blocks in reading order
    pub b: Vec<BlockTupleV1>,
}

/// Block tuple: [blockId, kindCode, textHash, text]
pub type BlockTupleV1 = (String, u16, String, String);
