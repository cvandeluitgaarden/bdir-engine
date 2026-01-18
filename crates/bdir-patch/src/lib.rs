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
pub mod telemetry;

pub use bdir_editpacket::schema::{EditPacketV1, BlockTupleV1};
pub use apply::{
    apply_patch_against_edit_packet,
    apply_patch_against_edit_packet_with_options,
    apply_patch_against_document,
    apply_patch_against_document_with_options,
    apply_patch_against_edit_packet_with_telemetry,
    apply_patch_against_document_with_telemetry,
};
pub use diagnostics::{DiagnosticCode, ValidationDiagnostic, ValidationError};
pub use telemetry::PatchTelemetry;
pub use schema::{OpType, PatchOpV1, PatchV1};
pub use validate::{
    KindCodePolicy,
    ValidateOptions,
    validate_patch,
    validate_patch_with_options,
    validate_patch_against_edit_packet,
    validate_patch_against_edit_packet_with_options,
    validate_patch_with_diagnostics,
    validate_patch_against_edit_packet_with_diagnostics,
    validate_patch_with_telemetry,
    validate_patch_against_edit_packet_with_telemetry,
};
