use clap::{Parser, Subcommand};
use std::fs;
use bdir_io::{core::Document, editpacket, patch};

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
    },
    ApplyPatch {
        /// Input Edit Packet JSON path (bdir-patch::EditPacketV1)
        edit_packet: String,
        /// Patch JSON path (bdir-patch::PatchV1)
        patch: String,
        /// Output minified JSON
        #[arg(long)]
        min: bool,
    },  
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Inspect { input, kind_filters, id, grep } => {
            let s = fs::read_to_string(&input)?;
            let mut doc: Document = serde_json::from_str(&s)?;

            // Keep output stable and useful for patch targeting/debugging.
            doc.recompute_hashes();

            let kind_ranges = parse_kind_filters(&kind_filters)?;

            // Header is stable and makes output self-describing.
            println!("blockId\tkindCode\ttextHash\tpreview");

            for b in &doc.blocks {
                if !kind_ranges.is_empty() && !kind_ranges.iter().any(|(lo, hi)| (lo..=hi).contains(&&b.kind_code)) {
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
                println!("{}\t{}\t{}\t{}", b.id, b.kind_code, b.text_hash, preview);
            }
        },
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
        },
        Command::ValidatePatch { edit_packet, patch } => {
            use std::process;

            let packet_s = match fs::read_to_string(&edit_packet) {
                Ok(s) => s,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let packet: editpacket::EditPacketV1 = match serde_json::from_str(&packet_s) {
                Ok(p) => p,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let patch_s = match fs::read_to_string(&patch) {
                Ok(s) => s,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let patch: patch::PatchV1 = match serde_json::from_str(&patch_s) {
                Ok(p) => p,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            match patch::validate_patch_against_edit_packet(&packet, &patch) {
                Ok(()) => { println!("OK"); process::exit(0); }
                Err(msg) => { eprintln!("{msg}"); process::exit(2); }
            }
        },
        Command::ApplyPatch { edit_packet, patch, min } => {
            use std::process;
                
            let packet_s = match fs::read_to_string(&edit_packet) {
                Ok(s) => s,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            // NOTE: deserialize using the canonical EditPacket type
            let packet: editpacket::EditPacketV1 = match serde_json::from_str(&packet_s) {
                Ok(p) => p,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let patch_s = match fs::read_to_string(&patch) {
                Ok(s) => s,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let patch: patch::PatchV1 = match serde_json::from_str(&patch_s) {
                Ok(p) => p,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let updated = match patch::apply_patch_against_edit_packet(&packet, &patch) {
                Ok(p) => p,
                Err(msg) => { eprintln!("{msg}"); process::exit(2); }
            };
        
            let out = if min {
                editpacket::to_minified_json(&updated).unwrap()
            } else {
                editpacket::to_pretty_json(&updated).unwrap()
            };
        
            // keep newline behavior stable
            println!("{out}");
            process::exit(0);
        },            
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

    // If we stopped because we hit the bound, check if there is any remaining non-whitespace.
    if !truncated {
        // We didn't hit the bound, so no ellipsis.
        return out;
    }
    if it.any(|c| !c.is_whitespace()) {
        // Ensure ellipsis does not exceed max_chars by replacing last char when needed.
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
