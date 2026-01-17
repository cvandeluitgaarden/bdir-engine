use bdir_core::model::Document;
use clap::{Parser, Subcommand};
use std::fs;
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
        Command::ValidatePatch { edit_packet, patch } => {
            use std::process;

            let packet_s = match fs::read_to_string(&edit_packet) {
                Ok(s) => s,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let packet: bdir_editpacket::EditPacketV1 = match serde_json::from_str(&packet_s) {
                Ok(p) => p,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let patch_s = match fs::read_to_string(&patch) {
                Ok(s) => s,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let patch: bdir_patch::PatchV1 = match serde_json::from_str(&patch_s) {
                Ok(p) => p,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            match bdir_patch::validate_patch_against_edit_packet(&packet, &patch) {
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
            let packet: bdir_editpacket::EditPacketV1 = match serde_json::from_str(&packet_s) {
                Ok(p) => p,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let patch_s = match fs::read_to_string(&patch) {
                Ok(s) => s,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let patch: bdir_patch::PatchV1 = match serde_json::from_str(&patch_s) {
                Ok(p) => p,
                Err(e) => { eprintln!("{e}"); process::exit(1); }
            };
        
            let updated = match bdir_patch::apply_patch_against_edit_packet(&packet, &patch) {
                Ok(p) => p,
                Err(msg) => { eprintln!("{msg}"); process::exit(2); }
            };
        
            let out = if min {
                bdir_editpacket::serialize::to_minified_json(&updated).unwrap()
            } else {
                bdir_editpacket::serialize::to_pretty_json(&updated).unwrap()
            };
        
            // keep newline behavior stable
            println!("{out}");
            process::exit(0);
        },            
    }

    Ok(())
}
