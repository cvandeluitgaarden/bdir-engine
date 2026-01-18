//! Protocol version and schema version constants used for RFC conformance.

/// BDIR Patch Protocol version supported by this crate.
///
/// This corresponds to the `v` field in Edit Packets and Patches.
pub const BDIR_PROTOCOL_V: u8 = 1;

/// Edit Packet wire format version.
pub const EDIT_PACKET_V: u8 = 1;

/// Patch wire format version.
pub const PATCH_V: u8 = 1;

/// JSON Schema bundle version for on-disk schemas under `spec/schemas/`.
///
/// Bump this if the schema constraints change (even if `v` stays the same).
pub const SCHEMA_BUNDLE_V: u8 = 1;
