use bdir_patch::{PatchV1, validate_patch};
use clap::{Parser, Subcommand};
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
            let doc_s = fs::read_to_string(&document)?;
            let mut doc: Document = serde_json::from_str(&doc_s)?;
            doc.recompute_hashes();

            let patch_s = fs::read_to_string(&patch)?;
            let patch: PatchV1 = serde_json::from_str(&patch_s)?;

            validate_patch(&doc, &patch).map_err(anyhow::Error::msg)?;

            // Intentionally minimal output; stable for scripts.
            println!("OK");
        }
    }

    Ok(())
}
