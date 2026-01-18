use clap::{Parser, Subcommand};
use std::fs;

use bdir_io::{core::Document, editpacket, patch};
use std::io::{self, IsTerminal, Write};

const INSPECT_PREVIEW_MAX_CHARS: usize = 80;

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
            let mut doc: Document = serde_json::from_str(&s)?;

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
            let mut doc: Document = serde_json::from_str(&s)?;
            doc.recompute_hashes();
            let packet = editpacket::from_document(&doc, tid);

            let out = if min {
                editpacket::to_minified_json(&packet)?
            } else {
                editpacket::to_pretty_json(&packet)?
            };

            println!("{out}");
        }

        Command::ValidatePatch { edit_packet, patch, min_before_len } => {
            use std::process;

            let packet_s = match fs::read_to_string(&edit_packet) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let packet: editpacket::EditPacketV1 = match serde_json::from_str(&packet_s) {
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

            let patch: patch::PatchV1 = match serde_json::from_str(&patch_s) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let opts = match min_before_len {
                Some(n) => patch::ValidateOptions { min_before_len: n },
                None => patch::ValidateOptions::default(),
            };

            match patch::validate_patch_against_edit_packet_with_options(&packet, &patch, opts) {
                Ok(()) => {
                    println!("OK");
                    process::exit(0);
                }
                Err(msg) => {
                    eprintln!("{msg}");
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
        } => {
            use std::process;

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

                let mut doc: Document = match serde_json::from_str(&doc_s) {
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

                let patch: patch::PatchV1 = match serde_json::from_str(&patch_s) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                };

                let updated = match patch::apply_patch_against_document(&doc, &patch) {
                    Ok(d) => d,
                    Err(msg) => {
                        eprintln!("{msg}");
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

            let packet: editpacket::EditPacketV1 = match serde_json::from_str(&packet_s) {
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

            let patch: patch::PatchV1 = match serde_json::from_str(&patch_s) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let updated = match patch::apply_patch_against_edit_packet(&packet, &patch) {
                Ok(p) => p,
                Err(msg) => {
                    eprintln!("{msg}");
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
