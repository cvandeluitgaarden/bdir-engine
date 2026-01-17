use clap::{Parser, Subcommand};
use std::process;
use std::fs;

use bdir_core::model::Document;
use bdir_editpacket::{convert::from_document, serialize};

#[derive(Debug, Parser)]
#[command(name = "bdir", version, about = "BDIR Patch Protocol MVP CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
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
        /// Input Document JSON path (bdir-core::Document)
        document: String,
        /// Patch JSON path (bdir-patch::PatchV1)
        patch: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::EditPacket { input, tid, min } => {
            let s = fs::read_to_string(&input)?;
            let mut doc: Document = serde_json::from_str(&s)?;
            doc.recompute_hashes();
            let packet = from_document(&doc, tid);

            let out = if min {
                serialize::to_minified_json(&packet)?
            } else {
                serialize::to_pretty_json(&packet)?
            };

            println!("{out}");
        },
        Command::ValidatePatch { document, patch } => {
            let doc_s = match fs::read_to_string(&document) {
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

            // Ensure hashes are consistent with current text.
            doc.recompute_hashes();

            let patch_s = match fs::read_to_string(&patch) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            let patch: bdir_patch::PatchV1 = match serde_json::from_str(&patch_s) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            match bdir_patch::validate_patch(&doc, &patch) {
                Ok(()) => {
                    println!("OK");
                    process::exit(0);
                }
                Err(msg) => {
                    // Exact error string, stable for CI / integrations.
                    eprintln!("{msg}");
                    process::exit(2);
                }
            }
        }   
    }

    Ok(())
}
