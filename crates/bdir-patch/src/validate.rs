use bdir_core::model::Document;
use std::collections::HashMap;
use bdir_core::hash::normalize_nfc;

use crate::{
    EditPacketV1,
    PatchTelemetry,
    diagnostics::{DiagnosticCode, ValidationDiagnostic, ValidationError},
    schema::{DeleteOccurrence, Occurrence, OpType, PatchV1},
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

    /// Require an explicit in-band page-hash binding in the patch (`h` + `ha`).
    ///
    /// When true, validators MUST reject patches that omit `h` or `ha`, even if an
    /// out-of-band `expected_page_hash` is available.
    pub strict_page_hash_binding: bool,
}

impl Default for ValidateOptions {
    fn default() -> Self {
        // Conservative default (matches pre-feature behavior).
        Self {
            min_before_len: 8,
            strict_kind_code: false,
            kind_code_policy: KindCodePolicy::default(),
            expected_page_hash: None,
            strict_page_hash_binding: false,
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

    // Strict page-hash binding (safety hardening):
    // In strict mode, a patch MUST carry an explicit in-band binding (`h` + `ha`).
    if opts.strict_page_hash_binding {
        if patch.h.is_none() {
            return Err(err_root(
                DiagnosticCode::PatchPageHashMissing,
                "h",
                "patch is missing required page hash binding (strict): include patch.h and patch.ha".to_string(),
            ));
        }
        let ha = patch.ha.as_deref().unwrap_or("").trim();
        if ha.is_empty() {
            return Err(err_root(
                DiagnosticCode::MissingField,
                "ha",
                "patch is missing required hash algorithm binding (strict): include patch.ha".to_string(),
            ));
        }
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

    // Hash algorithm binding (RFC-0001 v1.0.2):
    // - patch.ha identifies the algorithm used for patch.h
    // - patch.ha MAY be omitted for interoperability (algorithm is implied by the target document/packet)
    // - if patch.ha is present, receivers MUST reject when it does not match the target hash algorithm
    //
    // Note: this check only applies when the patch includes an in-band page-hash binding (`h`).
    // If the caller supplies `expected_page_hash` out-of-band, the hash algorithm is implied by
    // the target document/packet and `patch.ha` is ignored.
    if patch.h.is_some() {
        if let Some(patch_algo_raw) = patch.ha.as_deref() {
            let patch_algo = patch_algo_raw.trim().to_lowercase();
            if patch_algo.is_empty() {
                return Err(err_root(
                    DiagnosticCode::MissingField,
                    "ha",
                    "patch ha is empty".to_string(),
                ));
            }

            let doc_algo = doc.hash_algorithm.trim().to_lowercase();
            if patch_algo != doc_algo {
                return Err(err_root(
                    DiagnosticCode::HashAlgorithmMismatch,
                    "ha",
                    format!(
                        "patch hash algorithm mismatch (patch.ha='{}', doc.hash_algorithm='{}')",
                        patch_algo_raw, doc.hash_algorithm
                    ),
                ));
            }
        }
    }

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


// RFC-0001 v1.1 conflict rejection:
// Reject patches with conflicting mutating operations targeting the same block_id.
//
// - delete_block conflicts with any other op on the same block_id
// - replace_block conflicts with substring replace/delete on the same block_id
let mut seen: HashMap<&str, Vec<(usize, OpType)>> = HashMap::new();
for (i, op) in patch.ops.iter().enumerate() {
    seen.entry(op.block_id.as_str()).or_default().push((i, op.op));
}
for (block_id, opset) in &seen {
    let has_delete_block = opset.iter().any(|(_, t)| *t == OpType::DeleteBlock);
    if has_delete_block && opset.len() > 1 {
        let (i, _) = opset.iter().find(|(_, t)| *t != OpType::DeleteBlock).copied().unwrap_or(opset[0]);
        return Err(err_op(
            DiagnosticCode::ConflictingOperations,
            i,
            patch.ops[i].op,
            Some((*block_id).to_string()),
            Some(format!("ops[{i}].op")),
            format!("conflicting operations for block_id '{}' (delete_block cannot be combined with other ops)", block_id),
        ));
    }
    let has_replace_block = opset.iter().any(|(_, t)| *t == OpType::ReplaceBlock);
    let has_substring_mutation = opset.iter().any(|(_, t)| *t == OpType::Replace || *t == OpType::Delete);
    if has_replace_block && has_substring_mutation {
        let (i, _) = opset.iter().find(|(_, t)| *t == OpType::Replace || *t == OpType::Delete).copied().unwrap_or(opset[0]);
        return Err(err_op(
            DiagnosticCode::ConflictingOperations,
            i,
            patch.ops[i].op,
            Some((*block_id).to_string()),
            Some(format!("ops[{i}].op")),
            format!("conflicting operations for block_id '{}' (replace_block cannot be combined with substring replace/delete)", block_id),
        ));
    }
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
                let matches = count_non_overlapping(&block.text, before);
                if matches == 0 {
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

                // Ambiguity handling (RFC-0001 v1.0.2):
                // - If multiple matches exist and `occurrence` is omitted, reject.
                // - If `occurrence` is present, it must be a 1-indexed integer within range.
                match op.occurrence {
                    None => {
                        if matches > 1 {
                            return Err(err_op(
                                DiagnosticCode::BeforeAmbiguous,
                                i,
                                op.op,
                                Some(op.block_id.clone()),
                                Some(format!("ops[{i}].before")),
                                format!(
                                    "ops[{i}] (replace) before substring is ambiguous in block '{}' (matches {matches} times); provide occurrence",
                                    op.block_id
                                ),
                            ));
                        }
                    }
                    Some(Occurrence::Index(n)) => {
                        if n == 0 || (n as usize) > matches {
                            return Err(err_op(
                                DiagnosticCode::OccurrenceOutOfRange,
                                i,
                                op.op,
                                Some(op.block_id.clone()),
                                Some(format!("ops[{i}].occurrence")),
                                format!(
                                    "ops[{i}] (replace) occurrence out of range for block '{}' (occurrence={n}, matches={matches})",
                                    op.block_id
                                ),
                            ));
                        }
                    }
                    Some(Occurrence::Legacy(_)) => {
                        return Err(err_op(
                            DiagnosticCode::UnexpectedField,
                            i,
                            op.op,
                            Some(op.block_id.clone()),
                            Some(format!("ops[{i}].occurrence")),
                            format!(
                                "ops[{i}] (replace) invalid occurrence value (legacy string values are delete-only; use integer occurrence)",
                            ),
                        ));
                    }
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

                let matches = count_non_overlapping(&block.text, before);

                guard_before_diag(i, op.op, &op.block_id, before, opts.min_before_len)?;
                if matches == 0 {
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

                match op.occurrence {
                    None => {
                        if matches > 1 {
                            return Err(err_op(
                                DiagnosticCode::BeforeAmbiguous,
                                i,
                                op.op,
                                Some(op.block_id.clone()),
                                Some(format!("ops[{i}].before")),
                                format!(
                                    "ops[{i}] (delete) before substring is ambiguous in block '{}' (matches {matches} times); provide occurrence",
                                    op.block_id
                                ),
                            ));
                        }
                    }
                    Some(Occurrence::Index(n)) => {
                        if n == 0 || (n as usize) > matches {
                            return Err(err_op(
                                DiagnosticCode::OccurrenceOutOfRange,
                                i,
                                op.op,
                                Some(op.block_id.clone()),
                                Some(format!("ops[{i}].occurrence")),
                                format!(
                                    "ops[{i}] (delete) occurrence out of range for block '{}' (occurrence={n}, matches={matches})",
                                    op.block_id
                                ),
                            ));
                        }
                    }
                    // Legacy delete semantics are accepted for backwards compatibility.
                    Some(Occurrence::Legacy(DeleteOccurrence::First)) => {}
                    Some(Occurrence::Legacy(DeleteOccurrence::All)) => {}
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

                let new_block_id = op.new_block_id.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].new_block_id")),
                        format!("ops[{i}] (insert_after) missing new_block_id"),
                    )
                })?;
                if new_block_id.trim().is_empty() {
                    return Err(err_op(
                        DiagnosticCode::ContentEmpty,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].new_block_id")),
                        format!("ops[{i}] (insert_after) new_block_id is empty"),
                    ));
                }
                if doc.blocks.iter().any(|b| b.id == new_block_id) {
                    return Err(err_op(
                        DiagnosticCode::DuplicateBlockId,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].new_block_id")),
                        format!(
                            "ops[{i}] (insert_after) new_block_id '{}' already exists",
                            new_block_id
                        ),
                    ));
                }

                let _kind_code = op.kind_code.ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].kind_code")),
                        format!("ops[{i}] (insert_after) missing kind_code"),
                    )
                })?;

                let text = op.text.as_deref().ok_or_else(|| {
                    err_op(
                        DiagnosticCode::MissingField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].text")),
                        format!("ops[{i}] (insert_after) missing text"),
                    )
                })?;
                if text.trim().is_empty() {
                    return Err(err_op(
                        DiagnosticCode::ContentEmpty,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].text")),
                        format!("ops[{i}] (insert_after) text is empty"),
                    ));
                }
            }

            
OpType::InsertBefore => {
    if op.occurrence.is_some() {
        return Err(err_op(
            DiagnosticCode::UnexpectedField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].occurrence")),
            format!(
                "ops[{i}] (insert_before) unexpected occurrence (only valid for delete)"
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
                "ops[{i}] (insert_before) unexpected before (insert_before must not include before/after)"
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
                "ops[{i}] (insert_before) unexpected after (insert_before must not include before/after)"
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
                "ops[{i}] (insert_before) unexpected message (insert_before is mutating; use suggest instead)"
            ),
        ));
    }

    let new_block_id = op.new_block_id.as_deref().ok_or_else(|| {
        err_op(
            DiagnosticCode::MissingField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].new_block_id")),
            format!("ops[{i}] (insert_before) missing new_block_id"),
        )
    })?;
    if new_block_id.trim().is_empty() {
        return Err(err_op(
            DiagnosticCode::ContentEmpty,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].new_block_id")),
            format!("ops[{i}] (insert_before) new_block_id is empty"),
        ));
    }
    if doc.blocks.iter().any(|b| b.id == new_block_id) {
        return Err(err_op(
            DiagnosticCode::DuplicateBlockId,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].new_block_id")),
            format!(
                "ops[{i}] (insert_before) new_block_id '{}' already exists",
                new_block_id
            ),
        ));
    }

    let _kind_code = op.kind_code.ok_or_else(|| {
        err_op(
            DiagnosticCode::MissingField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].kind_code")),
            format!("ops[{i}] (insert_before) missing kind_code"),
        )
    })?;

    let text = op.text.as_deref().ok_or_else(|| {
        err_op(
            DiagnosticCode::MissingField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].text")),
            format!("ops[{i}] (insert_before) missing text"),
        )
    })?;
    if text.trim().is_empty() {
        return Err(err_op(
            DiagnosticCode::ContentEmpty,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].text")),
            format!("ops[{i}] (insert_before) text is empty"),
        ));
    }
}

OpType::ReplaceBlock => {
    if op.occurrence.is_some() {
        return Err(err_op(
            DiagnosticCode::UnexpectedField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].occurrence")),
            format!("ops[{i}] (replace_block) unexpected occurrence"),
        ));
    }
    if op.before.is_some() {
        return Err(err_op(
            DiagnosticCode::UnexpectedField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].before")),
            format!("ops[{i}] (replace_block) unexpected before"),
        ));
    }
    if op.after.is_some() {
        return Err(err_op(
            DiagnosticCode::UnexpectedField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].after")),
            format!("ops[{i}] (replace_block) unexpected after"),
        ));
    }
    if op.new_block_id.is_some() {
        return Err(err_op(
            DiagnosticCode::UnexpectedField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].new_block_id")),
            format!("ops[{i}] (replace_block) unexpected new_block_id"),
        ));
    }
    if op.message.is_some() {
        return Err(err_op(
            DiagnosticCode::UnexpectedField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].message")),
            format!("ops[{i}] (replace_block) unexpected message"),
        ));
    }

    let text = op.text.as_deref().ok_or_else(|| {
        err_op(
            DiagnosticCode::MissingField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].text")),
            format!("ops[{i}] (replace_block) missing text"),
        )
    })?;
    if text.trim().is_empty() {
        return Err(err_op(
            DiagnosticCode::ContentEmpty,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}].text")),
            format!("ops[{i}] (replace_block) text is empty"),
        ));
    }
}

OpType::DeleteBlock => {
    if op.occurrence.is_some()
        || op.before.is_some()
        || op.after.is_some()
        || op.new_block_id.is_some()
        || op.kind_code.is_some()
        || op.text.is_some()
        || op.message.is_some()
    {
        return Err(err_op(
            DiagnosticCode::UnexpectedField,
            i,
            op.op,
            Some(op.block_id.clone()),
            Some(format!("ops[{i}]")),
            format!("ops[{i}] (delete_block) contains fields that are not permitted"),
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
                if op.text.is_some() || op.new_block_id.is_some() || op.kind_code.is_some() {
                    return Err(err_op(
                        DiagnosticCode::UnexpectedField,
                        i,
                        op.op,
                        Some(op.block_id.clone()),
                        Some(format!("ops[{i}].text")),
                        format!(
                            "ops[{i}] (suggest) unexpected insert_after fields (suggest is non-mutating; use insert_after instead)"
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

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Count non-overlapping occurrences of `needle` in `haystack`.
///
/// This is used for ambiguity detection and occurrence range validation.
fn count_non_overlapping(haystack: &str, needle: &str) -> usize {
    // RFC-0001 §2.2: substring matching is performed over NFC-normalized strings.
    let haystack = normalize_nfc(haystack);
    let needle = normalize_nfc(needle);

    if needle.is_empty() {
        return 0;
    }

    let mut count = 0usize;
    let mut start = 0usize;
    while let Some(pos) = haystack[start..].find(&needle) {
        count += 1;
        start += pos + needle.len();
        if start >= haystack.len() {
            break;
        }
    }
    count
}

/// Validate `before` safety constraints and return a structured diagnostic on failure.
fn guard_before_diag(
    op_index: usize,
    op: OpType,
    block_id: &str,
    before: &str,
    min_before_len: usize,
) -> Result<(), ValidationError> {
    // RFC-0001 §2.2: operation strings are NFC-normalized.
    let before_nfc = normalize_nfc(before);

    if before_nfc.trim().is_empty() {
        return Err(err_op(
            DiagnosticCode::BeforeEmpty,
            op_index,
            op,
            Some(block_id.to_string()),
            Some(format!("ops[{op_index}].before")),
            format!("ops[{op_index}] before is empty"),
        ));
    }

    // Use char count to avoid surprising behavior with non-ASCII input.
    if before_nfc.chars().count() < min_before_len {
        return Err(err_op(
            DiagnosticCode::BeforeTooShort,
            op_index,
            op,
            Some(block_id.to_string()),
            Some(format!("ops[{op_index}].before")),
            format!(
                "ops[{op_index}] before is too short (<{min_before_len} chars); likely ambiguous"
            ),
        ));
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


// -----------------------------------------------------------------------------
// Edit Packet validators
// -----------------------------------------------------------------------------

/// Validate a patch against an Edit Packet.
///
/// This is the preferred validation surface for AI pipelines because it avoids
/// requiring reconstruction of a full `Document`.
pub fn validate_patch_against_edit_packet(packet: &EditPacketV1, patch: &PatchV1) -> Result<(), String> {
    validate_patch_against_edit_packet_with_options(packet, patch, ValidateOptions::default())
}

/// Validate a patch against an Edit Packet with configurable validator options.
pub fn validate_patch_against_edit_packet_with_options(
    packet: &EditPacketV1,
    patch: &PatchV1,
    opts: ValidateOptions,
) -> Result<(), String> {
    validate_patch_against_edit_packet_with_diagnostics(packet, patch, opts).map_err(|e| e.legacy_message())
}

/// Validate a patch against an Edit Packet and return structured diagnostics.
///
/// Note on page-hash binding:
/// - When validating against an Edit Packet, the packet's `h` value is authoritative.
/// - If the patch omits `h`, we bind it implicitly to the packet by defaulting
///   `expected_page_hash` to `packet.h`.
pub fn validate_patch_against_edit_packet_with_diagnostics(
    packet: &EditPacketV1,
    patch: &PatchV1,
    mut opts: ValidateOptions,
) -> Result<(), ValidationError> {
    if packet.v != 1 {
        return Err(err_root(
            DiagnosticCode::UnsupportedEditPacketVersion,
            "v",
            format!("unsupported edit packet version {}", packet.v),
        ));
    }

    // Default the expected page hash to the edit packet's hash so patches may omit `h`

    // when the validator has access to the authoritative packet.
    //
    // In strict mode, callers can require an explicit in-band binding by setting
    // `opts.strict_page_hash_binding = true`.
    if !opts.strict_page_hash_binding && opts.expected_page_hash.is_none() {
        opts.expected_page_hash = Some(packet.h.clone());
    }

    let doc = Document {
        page_hash: packet.h.clone(),
        hash_algorithm: packet.ha.clone(),
        blocks: packet
            .b
            .iter()
            .map(|t| bdir_core::model::Block {
                id: t.0.clone(),
                kind_code: t.1,
                text_hash: t.2.clone(),
                text: t.3.clone(),
            })
            .collect(),
    };

    validate_patch_with_diagnostics(&doc, patch, opts)
}
