#![doc = r#"
⚠️ INTERNAL CRATE – NOT A STABLE API

This crate is an internal implementation detail of the BDIR project.

Do NOT depend on this crate directly.
Use `bdir-io` instead.
"#]

pub mod schema;
pub mod convert;
pub mod serialize;

pub use schema::{EditPacketV1, BlockTupleV1};
