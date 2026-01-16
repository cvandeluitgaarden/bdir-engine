use crate::schema::EditPacketV1;

/// Serialize as minified JSON (no whitespace).
pub fn to_minified_json(packet: &EditPacketV1) -> Result<String, serde_json::Error> {
    serde_json::to_string(packet)
}

/// Serialize as pretty JSON (for debugging).
pub fn to_pretty_json(packet: &EditPacketV1) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(packet)
}
