# Validation Fixtures (Reference, Non-normative)

This folder contains **reference fixtures** demonstrating expected validation and
application behavior for the BDIR Patch Protocol.

> **Non-normative**
>
> These fixtures are **not** part of the normative protocol definition.
> If anything here appears to conflict with RFC-0001, treat **RFC-0001** as the
> source of truth.

## Goals

- Provide a shared, reusable set of test inputs to improve interoperability.
- Make edge cases explicit so implementers can verify deterministic behavior.
- Document expected outcomes and link each fixture to relevant RFC sections.

## Fixture format

Each fixture is a single JSON document with:

- `id` — stable identifier (used in filenames)
- `title` — short human label
- `reference` — always `true` (these are non-normative)
- `links` — list of RFC sections / requirements the case relates to
- `packet` — an Edit Packet (input)
- `patch` — the proposed patch (input)
- `context` — optional validator/applicator context (e.g., current page hash)
- `expect` — expected outcome

The `expect` shape is intentionally small and implementation-agnostic:

- `expect.valid: true|false` — whether the inputs are expected to validate
- `expect.reason` — short machine-friendly string
- `expect.notes` — short human explanation

Implementations MAY add richer internal diagnostics, but SHOULD be able to map
results back to the fields above.

## Index

| Fixture | Summary |
|---|---|
| `v001-valid-application.json` | Valid patch applies cleanly |
| `v002-page-hash-mismatch.json` | Reject patch when page hash does not match |
| `v003-missing-blockid.json` | Reject patch op missing `block_id` |
| `v004-ambiguous-before-substring.json` | Reject ambiguous `before` substring (recommended) |
| `v005-suggest-advisory-only.json` | `suggest` is advisory and non-mutating |
| `v006-unicode-normalization-mismatch.json` | Reject when Unicode normalization differs (unless canonicalized consistently upstream) |
| `v007-omitted-ha-defaults-sha256.json` | `ha` omitted defaults to `sha256` |
| `v008-truncated-hashes-valid.json` | Truncated hashes (>= 8 hex chars) validate |
| `v009-truncated-hash-too-short.json` | Reject hashes shorter than 8 hex chars |
