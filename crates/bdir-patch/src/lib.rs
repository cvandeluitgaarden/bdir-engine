#![doc = r#"
⚠️ INTERNAL CRATE – NOT A STABLE API

This crate is an internal implementation detail of the BDIR project.

Do NOT depend on this crate directly.
Use `bdir-io` instead.
"#]

pub mod apply;
pub mod diagnostics;
pub mod schema;
pub mod validate;

pub use bdir_editpacket::schema::{EditPacketV1, BlockTupleV1};
pub use apply::{apply_patch_against_edit_packet, apply_patch_against_document};
pub use diagnostics::{DiagnosticCode, ValidationDiagnostic, ValidationError};
pub use schema::{OpType, PatchOpV1, PatchV1};
pub use validate::{
    ValidateOptions,
    validate_patch,
    validate_patch_with_options,
    validate_patch_against_edit_packet,
    validate_patch_against_edit_packet_with_options,
    validate_patch_with_diagnostics,
    validate_patch_against_edit_packet_with_diagnostics,
};
