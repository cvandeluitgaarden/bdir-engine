# bdir-engine

Reference implementation of the **BDIR Patch Protocol**.

`bdir-engine` provides building blocks to:
- generate **BDIR Edit Packets** for AI input
- **validate** AI-generated patches deterministically
- **apply** patches safely to block-structured document content

This repository implements the protocol defined in the `bdir-spec` repository.

---

## Status

- **Current:** early reference implementation (focus: correctness and determinism)
- **Goal:** stable libraries + a minimal CLI for inspection, validation, and patch application

---

## Implementation status

The normative definition of the protocol is specified in **RFC-0001**.

For current engine support, known gaps, and planned work, see:

- `docs/implementation-status.md`

---


## What is BDIR?

BDIR (Block-based Document Intermediate Representation) represents a document as an ordered list of semantic blocks, each with:
- a stable identifier
- a semantic classification (`kindCode`)
- canonical text content
- a text hash

The BDIR Patch Protocol constrains AI systems to propose **patch instructions** rather than rewriting content.

---

## JSON formats and casing

This engine works with three JSON-adjacent representations:

- **Document JSON (engine input/output)**: a verbose, stable format used by the CLI and libraries in this repo.
- **Edit Packet (AI input)**: the compact wire format defined by RFC-0001.
- **Patch (AI output)**: the patch wire format defined by RFC-0001.

### Canonical casing

- **Document JSON** uses **snake_case** field names (e.g. `page_hash`, `hash_algorithm`, `kind_code`).
- **Edit Packet** and **Patch** follow the RFC wire formats exactly, including their field names
  (e.g. Edit Packet keys like `v`, `tid`, `h`, `ha`, `b`, and Patch operation field `blockId`).

See RFC-0001 "Edit Packet" (Section 6) and "Patch Instructions" (Section 8) for the normative wire formats.

### Required top-level Document fields

When providing **Document JSON** to the engine, the following top-level fields are required:

- `hash_algorithm` *(string)*
- `blocks` *(array)*

The following top-level field is optional (it can be computed):

- `page_hash` *(string, optional)*

For each `blocks[]` entry:

- Required: `id` *(string)*, `kind_code` *(number)*, `text` *(string)*
- Optional: `text_hash` *(string, optional — can be computed)*

Minimal example:

```json
{
  "hash_algorithm": "xxh64",
  "blocks": [
    {
      "id": "p1",
      "kind_code": 2,
      "text": "This is a paragraph."
    }
  ]
}
```

Note: when `text_hash` and/or `page_hash` are missing or empty, tools in this repo may recompute them deterministically.


## Components

This workspace is split into small crates:

- **bdir-core**: core data model (blocks, documents, hashing)
- **bdir-codebook**: kindCode mappings and importance semantics
- **bdir-editpacket**: generate Edit Packets (minified or pretty)
- **bdir-patch**: patch model, validation, and deterministic apply
- **bdir-io**: JSON IO helpers and canonicalization utilities
- **bdir-cli**: command-line interface for inspection and patch workflows

---

## CLI (planned / minimal)

The CLI is intended to support basic workflows:

- Inspect and print blocks
- Emit an edit packet for AI input
- Validate a patch against an edit packet / document
- Apply a validated patch

Currently implemented commands:

### Inspect

Inspect a Document JSON and print a deterministic, tab-separated table of blocks:

```bash
bdir inspect <document.json>
```

Output columns:

* `blockId`
* `kindCode`
* `textHash`
* `preview` (bounded, whitespace-collapsed)

Filters:

```bash
# Filter by kindCode (repeatable; supports ranges)
bdir inspect document.json --kind 0 --kind 2-10

# Filter by exact block id
bdir inspect document.json --id p1

# Filter by substring match on text
bdir inspect document.json --grep typo
```

### Other commands

```bash
bdir edit-packet <document.json> [--min] [--tid <trace-id>]
bdir validate-patch <edit-packet.json> <patch.json>
bdir apply-patch <edit-packet.json> <patch.json> [--min]
```

Example (illustrative):

```bash
bdir inspect input.md
bdir edit-packet input.md --min > edit-packet.json
bdir validate-patch edit-packet.json patch.json
bdir apply input.md patch.json --out updated.md
```

---

## Safety model

`bdir-engine` treats AI output as untrusted.

Validation is expected to enforce:
- schema correctness
- referenced blocks exist
- required `before` substrings match exactly
- all-or-nothing patch application

---

## Versioning

This project is intended to track the `bdir-spec` protocol versions.
Breaking changes will use a major version bump.

---

## License

Apache-2.0. See `LICENSE` and `NOTICE`.

---

## Contributing

Contributions are welcome, especially:
- validation edge cases
- interoperability test vectors
- CLI usability improvements
- documentation and examples

If you’re proposing protocol changes, please open an issue in the `bdir-spec` repository first.
