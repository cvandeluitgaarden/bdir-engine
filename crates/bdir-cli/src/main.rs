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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::EditPacket { input, tid, min } => {
            let s = fs::read_to_string(&input)?;
            let doc: Document = serde_json::from_str(&s)?;

            let packet = from_document(&doc, tid);

            let out = if min {
                serialize::to_minified_json(&packet)?
            } else {
                serialize::to_pretty_json(&packet)?
            };

            println!("{out}");
        }
    }

    Ok(())
}
