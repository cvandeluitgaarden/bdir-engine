//! `bdir-io` is the single supported public entrypoint for the BDIR Patch Protocol wire types
//! and deterministic helpers (edit packet generation, patch validation, and patch
//! application).
//!
//! This crate intentionally contains **no** HTML extraction, crawling, or AI logic.
//! Those belong in higher layers. `bdir-io` focuses on:
//! - stable types
//! - canonical JSON
//! - hashing
//! - validation / application helpers

// -----------------------------------------------------------------------------
// Public API contract
// -----------------------------------------------------------------------------
//
// Consumers SHOULD import from `bdir_io::prelude::*`.
// Anything not re-exported via the prelude is considered internal and may change
// without notice.

// Re-export the canonical document model.
#[doc(hidden)]
pub mod core {
    pub use bdir_core::model::{Block, BlockId, Document};
    pub use bdir_core::hash::{canonicalize_text, xxh64_hex};
}

/// Deterministic JSON canonicalization helpers.
///
/// These utilities are used for stable hashing and cache keys.
pub mod canonical_json;

/// Hash helpers for canonical JSON and cache keys.
pub mod hashing;

/// Version constants for RFC conformance and CI gating.
pub mod version;

// Re-export edit packet schema + helpers.
#[doc(hidden)]
pub mod editpacket {
    pub use bdir_editpacket::schema::{BlockTupleV1, EditPacketV1};
    pub use bdir_editpacket::convert::from_document;
    pub use bdir_editpacket::serialize::{to_minified_json, to_pretty_json};
}

// Re-export patch schema + helpers.
#[doc(hidden)]
pub mod patch {
    pub use bdir_patch::schema::{OpType, PatchOpV1, PatchV1};
    pub use bdir_patch::{DiagnosticCode, ValidationDiagnostic, ValidationError};
    pub use bdir_patch::{
        apply_patch_against_edit_packet,
        apply_patch_against_edit_packet_with_options,
        apply_patch_against_document,
        apply_patch_against_document_with_options,
        KindCodePolicy,
        ValidateOptions,
        validate_patch,
        validate_patch_with_options,
        validate_patch_with_diagnostics,
        validate_patch_against_edit_packet,
        validate_patch_against_edit_packet_with_options,
        validate_patch_against_edit_packet_with_diagnostics,
    };
}

/// Convenience prelude for consumers.
///
/// This is the **only supported** import surface for external users.
pub mod prelude {
    pub use crate::core::{Block, BlockId, Document};
    pub use crate::editpacket::{BlockTupleV1, EditPacketV1};
    pub use crate::patch::{OpType, PatchOpV1, PatchV1};
    pub use crate::patch::{DiagnosticCode, ValidationDiagnostic, ValidationError};
    pub use crate::{canonical_json, hashing};
}

/// Internal validation helpers.
#[doc(hidden)]
pub mod validate {
    pub use bdir_patch::{
        KindCodePolicy,
        ValidateOptions,
        validate_patch,
        validate_patch_with_options,
        validate_patch_with_diagnostics,
        validate_patch_against_edit_packet,
        validate_patch_against_edit_packet_with_options,
        validate_patch_against_edit_packet_with_diagnostics,
        DiagnosticCode,
        ValidationDiagnostic,
        ValidationError,
    };
}

/// Internal application helpers.
#[doc(hidden)]
pub mod apply {
    pub use bdir_patch::{apply_patch_against_edit_packet, apply_patch_against_document};
}
