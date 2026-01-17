//! `bdir-io` is the single public entrypoint for the BDIR Patch Protocol wire types
//! and deterministic helpers (edit packet generation, patch validation, and patch
//! application).
//!
//! This crate intentionally contains **no** HTML extraction, crawling, or AI logic.
//! Those belong in higher layers. `bdir-io` focuses on:
//! - stable types
//! - canonical JSON
//! - hashing
//! - validation / application helpers

// Re-export the canonical document model.
pub mod core {
    pub use bdir_core::model::{Block, BlockId, Document};
    pub use bdir_core::hash::{canonicalize_text, xxh64_hex};
}

// Re-export edit packet schema + helpers.
pub mod editpacket {
    pub use bdir_editpacket::schema::{BlockTupleV1, EditPacketV1};
    pub use bdir_editpacket::convert::from_document;
    pub use bdir_editpacket::serialize::{to_minified_json, to_pretty_json};
}

// Re-export patch schema + helpers.
pub mod patch {
    pub use bdir_patch::schema::{OpType, PatchOpV1, PatchV1};
    pub use bdir_patch::{
        apply_patch_against_edit_packet,
        validate_patch,
        validate_patch_against_edit_packet,
    };
}

/// Convenience prelude for consumers.
pub mod prelude {
    pub use crate::core::{Block, BlockId, Document};
    pub use crate::editpacket::{BlockTupleV1, EditPacketV1};
    pub use crate::patch::{OpType, PatchOpV1, PatchV1};
}
