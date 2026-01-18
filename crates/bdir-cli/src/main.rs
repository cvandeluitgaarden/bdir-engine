use clap::{Parser, Subcommand};
use std::fs;

use bdir_io::{core::Document, editpacket, patch};
use bdir_io::document_json::parse_document_json_str;
use jsonschema::Validator;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::io::{self, IsTerminal, Write};

const INSPECT_PREVIEW_MAX_CHARS: usize = 80;

// Compile the normative RFC schemas once per process.
//
// NOTE: Using include_str! keeps the CLI deterministic and avoids depending on a
// working directory at runtime.
static EDIT_PACKET_V1_SCHEMA: Lazy<Validator> = Lazy::new(|| {
    let schema: Value = serde_json::from_str(include_str!("../../../spec/schemas/edit-packet.v1.schema.json"))
        .expect("invalid embedded edit-packet.v1.schema.json");
    Validator::new(&schema).expect("failed to compile edit packet schema")
});

static PATCH_V1_SCHEMA: Lazy<Validator> = Lazy::new(|| {
    let schema: Value = serde_json::from_str(include_str!("../../../spec/schemas/patch.v1.schema.json"))
        .expect("invalid embedded patch.v1.schema.json");
    Validator::new(&schema).expect("failed to compile patch schema")
});

#[derive(Debug, Parser)]
#[command(name = "bdir", version, about = "BDIR Patch Protocol MVP CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect a Document JSON and print blocks in a deterministic tabular format.
    Inspect {
        /// Input Document JSON path (bdir-core::Document)
        input: String,

        /// Filter by kindCode (repeatable). Supports single values and ranges like `2-5`.
        #[arg(long = "kind")]
        kind_filters: Vec<String>,

        /// Filter by exact block id.
        #[arg(long)]
        id: Option<String>,

        /// Filter by substring match on block text.
        #[arg(long)]
        grep: Option<String>,
    },

    /// Convert a Document JSON into an Edit Packet JSON.
    EditPacket {
        /// Input Document JSON path (bdir-core::Document)
        input: String,
        /// Optional trace id to include in the packet
        #[arg(long)]
        tid: Option<String>,
        /// Output minified JSON
        #[arg(long)]
        min: bool,
    },

    ValidatePatch {
        /// Input Edit Packet JSON path (bdir-patch::EditPacketV1)
        edit_packet: String,
        /// Patch JSON path (bdir-patch::PatchV1)
        patch: String,

        /// Minimum length for `before` substrings used by replace/delete operations.
        ///
        /// Default is conservative (currently 8). Lowering this allows short fixes like
        /// "teh" -> "the" at the expense of potential ambiguity.
        #[arg(long = "min-before-len")]
        min_before_len: Option<usize>,

        /// Enable strict kindCode policy enforcement.
        ///
        /// When enabled, validation rejects any op targeting a block whose kindCode
        /// is not allowed. Defaults to allowing kindCodes 0-19 (Core + Medium) and
        /// allowing `suggest` ops on any kindCode.
        #[arg(long = "strict-kindcode")]
        strict_kindcode: bool,

        /// Allowed kindCode filter/ranges (repeatable) used only when --strict-kindcode is set.
        ///
        /// Supports single values and ranges like `2-5`, `0..19`, `0..=19`.
        /// If omitted, the default policy is 0-19.
        #[arg(long = "kindcode-allow")]
        kindcode_allow: Vec<String>,

        


        /// Emit PatchTelemetry JSON to stderr (deterministic, machine-readable).
        #[arg(long = "telemetry-json")]
        telemetry_json: bool,
        /// Print machine-readable JSON diagnostics to stderr on validation failure.
        #[arg(long = "diagnostics-json")]
        diagnostics_json: bool,
    },

    /// Apply a Patch.
    ///
    /// Backward-compatible (Edit Packet in/out):
    ///   bdir apply-patch <edit-packet.json> <patch.json> [--min]
    ///
    /// Document JSON in/out:
    ///   bdir apply-patch --doc <input.document.json> --patch <patch.json> --out <updated.document.json> [--min]
    ApplyPatch {
        /// Input Edit Packet JSON path (bdir-patch::EditPacketV1)
        edit_packet: Option<String>,
        /// Patch JSON path (bdir-patch::PatchV1)
        patch_pos: Option<String>,

        /// Input Document JSON path (bdir-core::Document)
        #[arg(long)]
        doc: Option<String>,

        /// Patch JSON path (bdir-patch::PatchV1)
        #[arg(long = "patch")]
        patch_flag: Option<String>,

        /// Output file path (required for --doc mode). If omitted, prints to stdout.
        #[arg(long)]
        out: Option<String>,

        /// Output minified JSON
        #[arg(long)]
        min: bool,

        /// Enable strict kindCode policy enforcement during patch application.
        #[arg(long = "strict-kindcode")]
        strict_kindcode: bool,

        /// Allowed kindCode filter/ranges (repeatable) used only when --strict-kindcode is set.
        #[arg(long = "kindcode-allow")]
        kindcode_allow: Vec<String>,

        /// Emit PatchTelemetry JSON to stderr (deterministic, machine-readable).
        #[arg(long = "telemetry-json")]
        telemetry_json: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Inspect {
            input,
            kind_filters,
            id,
            grep,
        } => {
            let s = fs::read_to_string(&input)?;
            let mut doc: Document = parse_document_json_str(&s)?;

            // Keep output stable and useful for patch targeting/debugging.
            doc.recompute_hashes();

            let kind_ranges = parse_kind_filters(&kind_filters)?;

            // TSV when non-interactive (tests/pipes), aligned table when interactive (terminal).
            let stdout = io::stdout();
            let is_tty = stdout.is_terminal();

            let emit = |out: &mut dyn Write| -> anyhow::Result<()> {
                writeln!(out, "blockId\tkindCode\timportance\ttextHash\tpreview")?;
            
                for b in &doc.blocks {
                    if !kind_ranges.is_empty()
                        && !kind_ranges
                            .iter()
                            .any(|(lo, hi)| (lo..=hi).contains(&&b.kind_code))
                    {
                        continue;
                    }
                    if let Some(ref want) = id {
                        if &b.id != want {
                            continue;
                        }
                    }
                    if let Some(ref needle) = grep {
                        if !b.text.contains(needle) {
                            continue;
                        }
                    }

                    let preview = make_preview(&b.text, INSPECT_PREVIEW_MAX_CHARS);
                
                    writeln!(
                        out,
                        "{}\t{}\t{}\t{}\t{}",
                        b.id,
                        b.kind_code,
                        bdir_codebook::importance(b.kind_code),
                        b.text_hash,
                        preview
                    )?;
                }
            
                Ok(())
            };

            if is_tty {
                let mut out = tabwriter::TabWriter::new(stdout.lock());
                emit(&mut out)?;
                out.flush()?;
            } else {
                let mut out = stdout.lock();
                emit(&mut out)?;
            }
        }

        Command::EditPacket { input, tid, min } => {
            let s = fs::read_to_string(&input)?;
            let mut doc: Document = parse_document_json_str(&s)?;
            doc.recompute_hashes();
            let packet = editpacket::from_document(&doc, tid);

            let out = if min {
                editpacket::to_minified_json(&packet)?
            } else {
                editpacket::to_pretty_json(&packet)?
            };

            println!("{out}");
        }

        Command::ValidatePatch {
            edit_packet,
            patch,
            min_before_len,
            strict_kindcode,
            kindcode_allow,
            diagnostics_json,
            telemetry_json,
        } => {
            use std::process;

            let packet_s = match fs::read_to_string(&edit_packet) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let packet_val: Value = match serde_json::from_str(&packet_s) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };
            validate_json_or_exit(&EDIT_PACKET_V1_SCHEMA, &packet_val);

            let packet: editpacket::EditPacketV1 = match serde_json::from_value(packet_val) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let patch_s = match fs::read_to_string(&patch) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let patch_val: Value = match serde_json::from_str(&patch_s) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };
            validate_json_or_exit(&PATCH_V1_SCHEMA, &patch_val);

            let patch: patch::PatchV1 = match serde_json::from_value(patch_val) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let mut opts = patch::ValidateOptions::default();
            if let Some(n) = min_before_len {
                opts.min_before_len = n;
            }
            if strict_kindcode {
                opts.strict_kind_code = true;
                if !kindcode_allow.is_empty() {
                    let ranges = parse_kind_filters(&kindcode_allow).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        process::exit(1);
                    });
                    opts.kind_code_policy = patch::KindCodePolicy {
                        allow_ranges: ranges,
                        allow_suggest_any: true,
                    };
                }
            }

            let (res, tel) = patch::validate_patch_against_edit_packet_with_telemetry(&packet, &patch, opts);

            match res {
                Ok(()) => {
                    if telemetry_json {
                        // Deterministic telemetry for monitoring / CI.
                        eprintln!("{}", serde_json::to_string(&tel).unwrap());
                    }
                    println!("OK");
                    process::exit(0);
                }
                Err(diag) => {
                    if diagnostics_json && telemetry_json {
                        // Combined machine-readable report.
                        let report = serde_json::json!({"telemetry": tel, "diagnostics": diag});
                        eprintln!("{}", serde_json::to_string(&report).unwrap());
                    } else if diagnostics_json {
                        eprintln!("{}", serde_json::to_string(&diag).unwrap());
                    } else if telemetry_json {
                        // Keep stderr parseable: emit a single JSON object.
                        let report = serde_json::json!({"telemetry": tel, "error": diag.legacy_message()});
                        eprintln!("{}", serde_json::to_string(&report).unwrap());
                    } else {
                        eprintln!("{}", diag.legacy_message());
                    }
                    process::exit(2);
                }
            }
        }

        Command::ApplyPatch {
            edit_packet,
            patch_pos,
            doc,
            patch_flag,
            out,
            min,
            strict_kindcode,
            kindcode_allow,
            telemetry_json,
        } => {
            use std::process;

            let mut opts = patch::ValidateOptions::default();
            if strict_kindcode {
                opts.strict_kind_code = true;
                if !kindcode_allow.is_empty() {
                    let ranges = parse_kind_filters(&kindcode_allow).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        process::exit(1);
                    });
                    opts.kind_code_policy = patch::KindCodePolicy {
                        allow_ranges: ranges,
                        allow_suggest_any: true,
                    };
                }
            }

            if let Some(doc_path) = doc {
                // Document JSON pathway
                if edit_packet.is_some() {
                    eprintln!("cannot use <edit-packet> positional arg together with --doc");
                    process::exit(1);
                }

                let patch_path = patch_flag.or(patch_pos).unwrap_or_else(|| {
                    eprintln!("missing --patch <patch.json>");
                    process::exit(1);
                });

                let doc_s = match fs::read_to_string(&doc_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                };

                let mut doc: Document = match parse_document_json_str(&doc_s) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                };

                // Ensure hashes are deterministic + consistent with the patch's expectations.
                doc.recompute_hashes();

                let patch_s = match fs::read_to_string(&patch_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                };

                let patch_val: Value = match serde_json::from_str(&patch_s) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                };
                validate_json_or_exit(&PATCH_V1_SCHEMA, &patch_val);

                let patch: patch::PatchV1 = match serde_json::from_value(patch_val) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                };

                let (res, tel) = patch::apply_patch_against_document_with_telemetry(&doc, &patch, opts.clone());

                let updated = match res {
                    Ok(d) => {
                        if telemetry_json {
                            eprintln!("{}", serde_json::to_string(&tel).unwrap());
                        }
                        d
                    }
                    Err(msg) => {
                        if telemetry_json {
                            let report = serde_json::json!({"telemetry": tel, "error": msg});
                            eprintln!("{}", serde_json::to_string(&report).unwrap());
                        } else {
                            eprintln!("{msg}");
                        }
                        process::exit(2);
                    }
                };

                let out_json = if min {
                    serde_json::to_string(&updated).unwrap()
                } else {
                    serde_json::to_string_pretty(&updated).unwrap()
                };

                if let Some(out_path) = out {
                    if let Err(e) = fs::write(&out_path, out_json) {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                    process::exit(0);
                }

                println!("{out_json}");
                process::exit(0);
            }

            // Edit Packet pathway (backward compatible)
            let edit_packet_path = edit_packet.unwrap_or_else(|| {
                eprintln!("missing <edit-packet.json> positional arg (or use --doc)");
                process::exit(1);
            });

            let patch_path = patch_pos.or(patch_flag).unwrap_or_else(|| {
                eprintln!("missing <patch.json> positional arg (or use --patch)");
                process::exit(1);
            });

            let packet_s = match fs::read_to_string(&edit_packet_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let packet_val: Value = match serde_json::from_str(&packet_s) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };
            validate_json_or_exit(&EDIT_PACKET_V1_SCHEMA, &packet_val);

            let packet: editpacket::EditPacketV1 = match serde_json::from_value(packet_val) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let patch_s = match fs::read_to_string(&patch_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let patch_val: Value = match serde_json::from_str(&patch_s) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };
            validate_json_or_exit(&PATCH_V1_SCHEMA, &patch_val);

            let patch: patch::PatchV1 = match serde_json::from_value(patch_val) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let (res, tel) = patch::apply_patch_against_edit_packet_with_telemetry(&packet, &patch, opts);

            let updated = match res {
                Ok(p) => {
                    if telemetry_json {
                        eprintln!("{}", serde_json::to_string(&tel).unwrap());
                    }
                    p
                }
                Err(msg) => {
                    if telemetry_json {
                        let report = serde_json::json!({"telemetry": tel, "error": msg});
                        eprintln!("{}", serde_json::to_string(&report).unwrap());
                    } else {
                        eprintln!("{msg}");
                    }
                    process::exit(2);
                }
            };

            let out_json = if min {
                editpacket::to_minified_json(&updated).unwrap()
            } else {
                editpacket::to_pretty_json(&updated).unwrap()
            };

            println!("{out_json}");
            process::exit(0);
        }
    }

    Ok(())
}

fn validate_json_or_exit(validator: &Validator, instance: &Value) {
    let errors: Vec<_> = validator.iter_errors(instance).collect();
    if errors.is_empty() {
        return;
    }

    for err in errors {
        eprintln!("{err}");
    }
    std::process::exit(1);
}

fn make_preview(s: &str, max_chars: usize) -> String {
    // Deterministic, bounded preview: collapse all whitespace to single spaces, trim, then truncate.
    let mut out = String::with_capacity(s.len().min(max_chars));
    let mut prev_was_ws = true; // treat leading ws as trimmed
    let mut count: usize = 0;
    let mut truncated = false;

    let mut it = s.chars().peekable();
    while let Some(ch) = it.next() {
        if ch.is_whitespace() {
            prev_was_ws = true;
            continue;
        }
        if prev_was_ws && !out.is_empty() {
            if count >= max_chars {
                truncated = true;
                break;
            }
            out.push(' ');
            count += 1;
        }
        prev_was_ws = false;
        if count >= max_chars {
            truncated = true;
            break;
        }
        out.push(ch);
        count += 1;
    }

    if !truncated {
        return out;
    }
    if it.any(|c| !c.is_whitespace()) {
        if count >= max_chars {
            out.pop();
        }
        out.push('…');
    }
    out
}

fn parse_kind_filters(filters: &[String]) -> anyhow::Result<Vec<(u16, u16)>> {
    let mut out = Vec::new();
    for raw in filters {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }

        // Accept `a-b` and `a..b` and `a..=b`.
        if let Some((a, b)) = s.split_once('-') {
            let lo: u16 = a.trim().parse()?;
            let hi: u16 = b.trim().parse()?;
            out.push((lo.min(hi), lo.max(hi)));
            continue;
        }
        if let Some((a, b)) = s.split_once("..=") {
            let lo: u16 = a.trim().parse()?;
            let hi: u16 = b.trim().parse()?;
            out.push((lo.min(hi), lo.max(hi)));
            continue;
        }
        if let Some((a, b)) = s.split_once("..") {
            let lo: u16 = a.trim().parse()?;
            let hi: u16 = b.trim().parse()?;
            out.push((lo.min(hi), lo.max(hi)));
            continue;
        }

        // Single value.
        let v: u16 = s.parse()?;
        out.push((v, v));
    }
    Ok(out)
}
