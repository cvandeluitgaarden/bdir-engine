use crate::schema::OpType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Deterministic, machine-readable telemetry for patch operations.
///
/// Notes:
/// - Contains *no* wall-clock timestamps (to preserve determinism).
/// - Intended for operational monitoring, CI, and cost/complexity analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchTelemetry {
    /// Operation category, e.g. "validate" or "apply".
    pub op: String,

    /// Whether the operation succeeded.
    pub ok: bool,

    /// Elapsed time (milliseconds).
    pub elapsed_ms: u64,

    /// Patch version.
    pub patch_v: u16,

    /// Edit Packet version (when applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_packet_v: Option<u16>,

    /// Hash algorithm (e.g., "xxh64", "sha256") when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_algorithm: Option<String>,

    /// Patch ops total.
    pub patch_ops: usize,

    /// Patch ops grouped by op type.
    pub patch_ops_by_type: BTreeMap<String, usize>,

    /// Unique block ids targeted by patch ops.
    pub target_blocks: usize,

    /// Whether strict kindCode policy was enabled.
    pub strict_kind_code: bool,

    /// Validator min_before_len.
    pub min_before_len: usize,

    /// kindCode allow ranges, formatted like "0-19".
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub kind_code_allow: Vec<String>,

    /// Total character count of input content (best-effort, deterministic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_chars: Option<usize>,

    /// Total character count of output content (best-effort, deterministic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_chars: Option<usize>,

    /// Optional machine-readable error code (when failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

impl PatchTelemetry {
    pub fn op_counts(ops: &[crate::schema::PatchOpV1]) -> (usize, BTreeMap<String, usize>, usize) {
        let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
        let mut targets: BTreeMap<String, ()> = BTreeMap::new();
        for o in ops {
            *by_type.entry(format!("{:?}", o.op).to_lowercase()).or_insert(0) += 1;
            targets.insert(o.block_id.clone(), ());
        }
        (ops.len(), by_type, targets.len())
    }

    pub fn kind_allow_strings(ranges: &[(u16, u16)]) -> Vec<String> {
        ranges.iter().map(|(lo, hi)| format!("{lo}-{hi}")).collect()
    }

    pub fn op_type_key(op: OpType) -> String {
        format!("{:?}", op).to_lowercase()
    }
}
