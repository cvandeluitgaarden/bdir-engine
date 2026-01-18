# Canonical large document fixture

This directory contains a **synthetic, deterministic** large Document JSON fixture intended for:

- CLI stress testing (e.g. `bdir inspect`, `bdir edit-packet`)
- Engine/hash/normalization regression checks
- Exercising block preview rendering, filtering, and stable ordering

## Files

- `document.json` — BDIR Document JSON with hundreds of blocks covering:
  - headings + long paragraphs
  - nested-list-like text
  - table-like rows
  - code blocks
  - repeated boilerplate-like blocks (`kind_code: 20`)

## Usage

From the workspace root:

```sh
cargo run -p bdir-cli -- inspect crates/bdir-cli/tests/fixtures/large-document/document.json > /tmp/inspect.tsv
cargo run -p bdir-cli -- edit-packet crates/bdir-cli/tests/fixtures/large-document/document.json --min > /tmp/edit-packet.json
```

The fixture uses placeholder `text_hash` values. The CLI recomputes hashes deterministically during processing.
