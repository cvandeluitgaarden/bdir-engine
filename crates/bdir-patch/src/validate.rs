use bdir_core::model::Document;
use bdir_codebook as codebook;

use crate::{
    EditPacketV1, PatchTelemetry, diagnostics::{DiagnosticCode, ValidationDiagnostic, ValidationError}, schema::{DeleteOccurrence, OpType, PatchV1}
};

/// kindCode enforcement policy.
///
/// When strict mode is enabled, patch validation rejects any op that targets a block
/// whose `kindCode` is not allowed by this policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindCodePolicy {
    /// Allowed inclusive kindCode ranges.
    ///
    /// Example: `[(0, 19)]` allows core + medium-importance content per RFC-0001.
    pub allow_ranges: Vec<(u16, u16)>,

    /// If true, `suggest` ops are allowed for any kindCode.
    ///
    /// This preserves the ability to attach non-mutating guidance to boilerplate/UI
    /// blocks while still blocking mutations.
    pub allow_suggest_any: bool,
}

impl Default for KindCodePolicy {
    fn default() -> Self {
        Self {
            // Conservative default aligned with RFC-0001 importance tiers.
            // 0–19 is Core (0–9) + Medium (10–19).
            allow_ranges: vec![(0, 19)],
            allow_suggest_any: true,
        }
    }
}

impl KindCodePolicy {
    fn allows(&self, op: OpType, kind_code: u16) -> bool {
        if op == OpType::Suggest && self.allow_suggest_any {
            return true;
        }
        self.allow_ranges
            .iter()
            .any(|(lo, hi)| (*lo..=*hi).contains(&kind_code))
    }
}

/// Validator configuration options.
///
/// These options exist to make safety / strictness trade-offs explicit and testable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateOptions {
    /// Minimum character length for `before` substrings.
    ///
    /// Short `before` strings can be ambiguous and match unintended parts of a block.
    pub min_before_len: usize,

    /// Enable strict kindCode policy enforcement.
    ///
    /// When true, validators MUST reject any op targeting a block whose kindCode is
    /// disallowed by `kind_code_policy`.
    pub strict_kind_code: bool,

    /// Policy used when `strict_kind_code` is enabled.
    ///
    /// Defaults to allowing kindCodes 0–19 (Core + Medium) and allowing `suggest` on any kindCode.
    pub kind_code_policy: KindCodePolicy,

    /// Expected page-level hash when the patch itself is not bound via `h`.
    ///
    /// When set, validators treat this as the required page hash binding and
    /// will reject patches whose `h` (if present) conflicts with it.
    #[allow(dead_code)]
    pub expected_page_hash: Option<String>,
}

impl Default for ValidateOptions {
    fn default() -> Self {
        // Conservative default (matches pre-feature behavior).
        Self {
            min_before_len: 8,
            strict_kind_code: false,
            kind_code_policy: KindCodePolicy::default(),
            expected_page_hash: None,
        }
    }
}

fn enforce_kind_code(
    i: usize,
    op: OpType,
    block_id: &str,
    kind_code: u16,
    opts: &ValidateOptions,
) -> Result<(), ValidationError> {
    if !opts.strict_kind_code {
        return Ok(());
    }

    if opts.kind_code_policy.allows(op, kind_code) {
        return Ok(());
    }

    let policy_summary = if opts.kind_code_policy.allow_ranges.is_empty() {
        "allow_ranges=[]".to_string()
    } else {
        let ranges = opts
            .kind_code_policy
            .allow_ranges
            .iter()
            .map(|(lo, hi)| format!("{lo}-{hi}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "allow_ranges=[{ranges}], allow_suggest_any={}",
            opts.kind_code_policy.allow_suggest_any
        )
    };

    Err(err_op(
        DiagnosticCode::KindCodeDisallowed,
        i,
        op,
        Some(block_id.to_string()),
        Some(format!("ops[{i}].block_id")),
        format!(
            "ops[{i}] targets kindCode {kind_code}, which is disallowed under strict kindCode policy ({policy_summary})"
        ),
    ))
}

/// Validate a patch against a document. Strict and fail-fast.
///
/// Rules:
/// - patch version must be supported
/// - block_id must exist
/// - required fields must be present per op
/// - `before` (when required) must be found in the block text
/// - optional guard: reject very short `before` strings (ambiguity)
pub fn validate_patch(doc: &Document, patch: &PatchV1) -> Result<(), String> {
    validate_patch_with_options(doc, patch, ValidateOptions::default())
}

/// Validate a patch against a document with configurable validator options.
pub fn validate_patch_with_options(
    doc: &Document,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> Result<(), String> {
    validate_patch_with_diagnostics(doc, patch, opts).map_err(|e| e.legacy_message())
}

/// Validate a patch against a document and return structured diagnostics.
pub fn validate_patch_with_diagnostics(
    doc: &Document,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> Result<(), ValidationError> {
    if patch.v != 1 {
        return Err(err_root(
            DiagnosticCode::UnsupportedPatchVersion,
            "v",
            format!("unsupported patch version {}", patch.v),
        ));
    }
    // Safety binding: ensure the patch is only applied to the intended page version.
    //
    // A patch MUST be bound to a specific page hash either by including `h` in the patch,
    // or by the caller providing an explicit `expected_page_hash` out-of-band.
    let expected = match (patch.h.as_deref(), opts.expected_page_hash.as_deref()) {
        (Some(patch_h), Some(expected_h)) => {
            if patch_h != expected_h {
                return Err(err_root(
                    DiagnosticCode::PatchPageHashMismatch,
                    "h",
                    format!(
                        "patch page hash mismatch (patch.h='{}' differs from expected_page_hash='{}')",
                        patch_h, expected_h
                    ),
                ));
            }
            patch_h
        }
        (Some(patch_h), None) => patch_h,
        (None, Some(expected_h)) => expected_h,
        (None, None) => {
            return Err(err_root(
                DiagnosticCode::PatchPageHashMissing,
                "h",
                "patch is missing required page hash binding: include patch.h or provide expected_page_hash".to_string(),
            ));
        }
    };

    if doc.page_hash != expected {
        return Err(err_root(
            DiagnosticCode::PatchPageHashMismatch,
            "h",
            format!(
                "patch page hash mismatch (expected '{}', got '{}')",
                expected, doc.page_hash
            ),
        ));
    }

    for (i, op) in patch.ops.iter().enumerate() {
        let block = doc
            .blocks
            .iter()
            .find(|b| b.id == op.block_id)
            .ok_or_else(|| {
                err_op(
                    DiagnosticCode::UnknownBlockId,
                    i,
                    op.op,
                    Some(op.block_id.clone()),
                    Some(format!("ops[{i}].block_id")),
                    format!("ops[{i}] references unknown block_id '{}'", op.block_id),
                )
            })?;

        // Optional strict safety gate: enforce kindCode policy.
        enforce_kind_code(i, op.op, &op.block_id, block.kind_code, &opts)?;

        match op.op {
            OpType::Replace => {
                if op.occurrence.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].occurrence")),
                        format!("ops[{i}] (replace) unexpected occurrence (only valid for delete)"),
                    ));
                }
                let before = op.before.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!("ops[{i}] (replace) missing before"),
                    )
                })?;
                let _after = op.after.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].after")),
                        format!("ops[{i}] (replace) missing after"),
                    )
                })?;

                guard_before_diag(i, op.op, &op.block_id, before, opts.min_before_len)?;
                if !block.text.contains(before) {
                    return Err(err_op(
                        DiagnosticCode::BeforeNotFound,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!(
                            "ops[{i}] (replace) before substring not found in block '{}'",
                            op.block_id
                        ),
                    ));
                }
            }

            OpType::Delete => {
                let before = op.before.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!("ops[{i}] (delete) missing before"),
                    )
                })?;

                let occ = op.occurrence.unwrap_or(DeleteOccurrence::All);

                let _ = match occ {
                    DeleteOccurrence::First | DeleteOccurrence::All => occ,
                };

                guard_before_diag(i, op.op, &op.block_id, before, opts.min_before_len)?;
                if !block.text.contains(before) {
                    return Err(err_op(
                        DiagnosticCode::BeforeNotFound,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!(
                            "ops[{i}] (delete) before substring not found in block '{}'",
                            op.block_id
                        ),
                    ));
                }
            }

            OpType::InsertAfter => {
                if op.occurrence.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].occurrence")),
                        format!(
                            "ops[{i}] (insert_after) unexpected occurrence (only valid for delete)"
                        ),
                    ));
                }
                if op.before.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!(
                            "ops[{i}] (insert_after) unexpected before (insert_after must not include before/after)"
                        ),
                    ));
                }
                if op.after.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].after")),
                        format!(
                            "ops[{i}] (insert_after) unexpected after (insert_after must not include before/after)"
                        ),
                    ));
                }
                if op.message.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].message")),
                        format!(
                            "ops[{i}] (insert_after) unexpected message (insert_after is mutating; use suggest instead)"
                        ),
                    ));
                }
                let content = op.content.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].content")),
                        format!("ops[{i}] (insert_after) missing content"),
                    )
                })?;
                if content.trim().is_empty() {
                    return Err(err_op(
                        DiagnosticCode::ContentEmpty,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].content")),
                        format!("ops[{i}] (insert_after) content is empty"),
                    ));
                }
            }

            OpType::Suggest => {
                if op.occurrence.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].occurrence")),
                        format!(
                            "ops[{i}] (suggest) unexpected occurrence (only valid for delete)"
                        ),
                    ));
                }
                if op.before.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!(
                            "ops[{i}] (suggest) unexpected before (suggest must not include before/after)"
                        ),
                    ));
                }
                if op.after.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].after")),
                        format!(
                            "ops[{i}] (suggest) unexpected after (suggest must not include before/after)"
                        ),
                    ));
                }
                if op.content.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].content")),
                        format!(
                            "ops[{i}] (suggest) unexpected content (suggest is non-mutating; use insert_after instead)"
                        ),
                    ));
                }
                let msg = op.message.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].message")),
                        format!("ops[{i}] (suggest) missing message"),
                    )
                })?;
                if msg.trim().is_empty() {
                    return Err(err_op(
                        DiagnosticCode::MessageEmpty,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].message")),
                        format!("ops[{i}] (suggest) message is empty"),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn guard_before_diag(
    i: usize,
    op: OpType,
    block_id: &str,
    before: &str,
    min_before_len: usize,
) -> Result<(), ValidationError> {
    if before.trim().is_empty() {
        return Err(err_op(
            DiagnosticCode::BeforeEmpty,
            i,
            op,
            Some(block_id.to_string()),
            Some(format!("ops[{i}].before")),
            format!("ops[{i}] before is empty/whitespace"),
        ));
    }
    if before.chars().count() < min_before_len {
        return Err(err_op(
            DiagnosticCode::BeforeTooShort,
            i,
            op,
            Some(block_id.to_string()),
            Some(format!("ops[{i}].before")),
            format!(
                "ops[{i}] before is too short (<{min_before_len} chars); likely ambiguous"
            ),
        ));
    }
    Ok(())
}

pub fn validate_patch_against_edit_packet(packet: &EditPacketV1, patch: &PatchV1) -> Result<(), String> {
    validate_patch_against_edit_packet_with_options(packet, patch, ValidateOptions::default())
}

/// Validate a patch against an edit packet with configurable validator options.
pub fn validate_patch_against_edit_packet_with_options(
    packet: &EditPacketV1,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> Result<(), String> {
    validate_patch_against_edit_packet_with_diagnostics(packet, patch, opts)
        .map_err(|e| e.legacy_message())
}

/// Validate a patch against an edit packet and return structured diagnostics.
pub fn validate_patch_against_edit_packet_with_diagnostics(
    packet: &EditPacketV1,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> Result<(), ValidationError> {
    if patch.v != 1 {
        return Err(err_root(
            DiagnosticCode::UnsupportedPatchVersion,
            "v",
            format!("unsupported patch version {}", patch.v),
        ));
    }
    if packet.v != 1 {
        return Err(err_root(
            DiagnosticCode::UnsupportedEditPacketVersion,
            "v",
            format!("unsupported edit packet version {}", packet.v),
        ));
    }
    // Safety binding: ensure the patch is only applied to the intended page version.
    //
    // A patch MUST be bound to a specific page hash either by including `h` in the patch,
    // or by the caller providing an explicit `expected_page_hash` out-of-band.
    let expected = match (patch.h.as_deref(), opts.expected_page_hash.as_deref()) {
        (Some(patch_h), Some(expected_h)) => {
            if patch_h != expected_h {
                return Err(err_root(
                    DiagnosticCode::PatchPageHashMismatch,
                    "h",
                    format!(
                        "patch page hash mismatch (patch.h='{}' differs from expected_page_hash='{}')",
                        patch_h, expected_h
                    ),
                ));
            }
            patch_h
        }
        (Some(patch_h), None) => patch_h,
        (None, Some(expected_h)) => expected_h,
        (None, None) => {
            return Err(err_root(
                DiagnosticCode::PatchPageHashMissing,
                "h",
                "patch is missing required page hash binding: include patch.h or provide expected_page_hash".to_string(),
            ));
        }
    };

    if packet.h != expected {
        return Err(err_root(
            DiagnosticCode::PatchPageHashMismatch,
            "h",
            format!(
                "patch page hash mismatch (expected '{}', got '{}')",
                expected, packet.h
            ),
        ));
    }

    for (i, op) in patch.ops.iter().enumerate() {
        let (block_idx, block) = packet
            .b
            .iter()
            .enumerate()
            .find(|(_, t)| t.0 == op.block_id)
            .ok_or_else(|| {
            err_op(
                DiagnosticCode::UnknownBlockId,
                i,
                op.op,
                Some(op.block_id.clone()),
                Some(format!("ops[{i}].block_id")),
                format!("ops[{i}] references unknown block_id '{}'", op.block_id),
            )
        })?;

        // RFC-0001 kind_code semantics define canonical v1 importance ranges.
        // If the edit packet contains a non-canonical kind_code, reject early.
        if !codebook::is_valid_v1(block.1) {
            return Err(err_op(
                DiagnosticCode::KindCodeOutOfRange,
                i,
                op.op,
                Some(op.block_id.clone()),
                Some(format!("b[{block_idx}][1]")),
                format!(
                    "edit packet block '{}' has non-canonical kind_code {} (expected 0-59 or 99)",
                    op.block_id, block.1
                ),
            ));
        }

        // Optional strict safety gate: enforce kindCode policy.
        // tuple layout: (id, kind, text_hash, text)
        enforce_kind_code(i, op.op, &op.block_id, block.1, &opts)?;

        // tuple layout: (id, kind, text_hash, text)
        let block_text = &block.3;

        match op.op {
            OpType::Replace => {
                if op.occurrence.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].occurrence")),
                        format!("ops[{i}] (replace) unexpected occurrence (only valid for delete)"),
                    ));
                }
                let before = op.before.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!("ops[{i}] (replace) missing before"),
                    )
                })?;
                let _after = op.after.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].after")),
                        format!("ops[{i}] (replace) missing after"),
                    )
                })?;

                guard_before_diag(i, op.op, &op.block_id, before, opts.min_before_len)?;
                if !block_text.contains(before) {
                    return Err(err_op(
                        DiagnosticCode::BeforeNotFound,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!(
                            "ops[{i}] (replace) before substring not found in block '{}'",
                            op.block_id
                        ),
                    ));
                }
            }

            OpType::Delete => {
                let before = op.before.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!("ops[{i}] (delete) missing before"),
                    )
                })?;

                let occ = op.occurrence.unwrap_or(DeleteOccurrence::All);

                let _ = match occ {
                    DeleteOccurrence::First | DeleteOccurrence::All => occ,
                };

                guard_before_diag(i, op.op, &op.block_id, before, opts.min_before_len)?;
                if !block_text.contains(before) {
                    return Err(err_op(
                        DiagnosticCode::BeforeNotFound,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!(
                            "ops[{i}] (delete) before substring not found in block '{}'",
                            op.block_id
                        ),
                    ));
                }
            }

            OpType::InsertAfter => {
                if op.occurrence.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].occurrence")),
                        format!(
                            "ops[{i}] (insert_after) unexpected occurrence (only valid for delete)"
                        ),
                    ));
                }
                if op.before.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!(
                            "ops[{i}] (insert_after) unexpected before (insert_after must not include before/after)"
                        ),
                    ));
                }
                if op.after.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].after")),
                        format!(
                            "ops[{i}] (insert_after) unexpected after (insert_after must not include before/after)"
                        ),
                    ));
                }
                if op.message.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].message")),
                        format!(
                            "ops[{i}] (insert_after) unexpected message (insert_after is mutating; use suggest instead)"
                        ),
                    ));
                }
                let content = op.content.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].content")),
                        format!("ops[{i}] (insert_after) missing content"),
                    )
                })?;
                if content.trim().is_empty() {
                    return Err(err_op(
                        DiagnosticCode::ContentEmpty,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].content")),
                        format!("ops[{i}] (insert_after) content is empty"),
                    ));
                }
            }

            OpType::Suggest => {
                if op.occurrence.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].occurrence")),
                        format!(
                            "ops[{i}] (suggest) unexpected occurrence (only valid for delete)"
                        ),
                    ));
                }
                if op.before.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].before")),
                        format!(
                            "ops[{i}] (suggest) unexpected before (suggest must not include before/after)"
                        ),
                    ));
                }
                if op.after.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].after")),
                        format!(
                            "ops[{i}] (suggest) unexpected after (suggest must not include before/after)"
                        ),
                    ));
                }
                if op.content.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].content")),
                        format!(
                            "ops[{i}] (suggest) unexpected content (suggest is non-mutating; use insert_after instead)"
                        ),
                    ));
                }
                let msg = op.message.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].message")),
                        format!("ops[{i}] (suggest) missing message"),
                    )
                })?;
                if msg.trim().is_empty() {
                    return Err(err_op(
                        DiagnosticCode::MessageEmpty,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].message")),
                        format!("ops[{i}] (suggest) message is empty"),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn err_root(code: DiagnosticCode, path: &str, message: String) -> ValidationError {
    ValidationError::single(ValidationDiagnostic {
        code,
        path: Some(path.to_string()),
        op_index: None,
        op: None,
        block_id: None,
        message,
    })
}

fn err_op(
    code: DiagnosticCode,
    op_index: usize,
    op: OpType,
    block_id: Option<String>,
    path: Option<String>,
    message: String,
) -> ValidationError {
    ValidationError::single(ValidationDiagnostic {
        code,
        path,
        op_index: Some(op_index),
        op: Some(op),
        block_id,
        message,
    })
}
// -----------------------------------------------------------------------------
// Telemetry wrappers (deterministic)
// -----------------------------------------------------------------------------

/// Validate a patch against an Edit Packet and return deterministic telemetry.
///
/// Returns a tuple of (result, telemetry) so callers can emit telemetry even on failure.
pub fn validate_patch_against_edit_packet_with_telemetry(
    packet: &EditPacketV1,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> (Result<(), ValidationError>, PatchTelemetry) {
    use std::time::Instant;

    let start = Instant::now();
    let (patch_ops, patch_ops_by_type, target_blocks) = PatchTelemetry::op_counts(&patch.ops);

    let input_chars = Some(packet.b.iter().map(|t| t.3.len()).sum::<usize>());

    let res = validate_patch_against_edit_packet_with_diagnostics(packet, patch, opts.clone());
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let tel = PatchTelemetry {
        op: "validate".to_string(),
        ok: res.is_ok(),
        elapsed_ms,
        patch_v: patch.v as u16,
        edit_packet_v: Some(packet.v as u16),
        hash_algorithm: Some(packet.ha.clone()),
        patch_ops,
        patch_ops_by_type,
        target_blocks,
        strict_kind_code: opts.strict_kind_code,
        min_before_len: opts.min_before_len,
        kind_code_allow: if opts.strict_kind_code {
            PatchTelemetry::kind_allow_strings(&opts.kind_code_policy.allow_ranges)
        } else {
            vec![]
        },
        input_chars,
        output_chars: None,
        error_code: res
            .as_ref()
            .err()
            .and_then(|e| e.diagnostics.first())
            .map(|d| format!("{:?}", d.code).to_lowercase()),
    };

    // Keep ordering stable (BTreeMap) and avoid unused mut lint in older rust versions.
    if tel.kind_code_allow.is_empty() {
        // no-op
    }

    (res.map(|_| ()), tel)
}

/// Validate a patch against a Document and return deterministic telemetry.
///
/// Returns a tuple of (result, telemetry) so callers can emit telemetry even on failure.
pub fn validate_patch_with_telemetry(
    doc: &Document,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> (Result<(), ValidationError>, PatchTelemetry) {
    use std::time::Instant;

    let start = Instant::now();
    let (patch_ops, patch_ops_by_type, target_blocks) = PatchTelemetry::op_counts(&patch.ops);
    let input_chars = Some(doc.blocks.iter().map(|b| b.text.len()).sum::<usize>());

    let res = validate_patch_with_diagnostics(doc, patch, opts.clone());
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let tel = PatchTelemetry {
        op: "validate".to_string(),
        ok: res.is_ok(),
        elapsed_ms,
        patch_v: patch.v as u16,
        edit_packet_v: None,
        hash_algorithm: Some(doc.hash_algorithm.clone()),
        patch_ops,
        patch_ops_by_type,
        target_blocks,
        strict_kind_code: opts.strict_kind_code,
        min_before_len: opts.min_before_len,
        kind_code_allow: if opts.strict_kind_code {
            PatchTelemetry::kind_allow_strings(&opts.kind_code_policy.allow_ranges)
        } else {
            vec![]
        },
        input_chars,
        output_chars: None,
        error_code: res
            .as_ref()
            .err()
            .and_then(|e| e.diagnostics.first())
            .map(|d| format!("{:?}", d.code).to_lowercase()),
    };

    (res.map(|_| ()), tel)
}
