# SemVer Policy

This project uses SemVer with an explicit stability boundary for protocol semantics.

## Patch releases (1.0.x)

Allowed:
- Documentation clarifications that do **not** change meaning
- Schema fixes that correct obvious errors without changing accepted/produced wire format
- Bug fixes that restore intended v1.0 behavior

Not allowed:
- Any semantic change to protocol operations
- New required fields or stricter validation that breaks valid v1.0 documents/patches

## Minor releases (1.1.x)

Allowed (must remain backward compatible with v1.0.x):
- New optional fields with safe defaults
- New optional operations that are safe to ignore
- Additional validation that only rejects previously *invalid/ambiguous* cases, provided v1.0.x valid inputs remain valid
- Clarifications that remove ambiguity without contradicting v1.0.2 semantics

Not allowed:
- Redefining the meaning of existing fields/operations
- Changing defaults in ways that alter outcomes for valid v1.0 patches
- Removing or renaming fields/ops in the canonical wire format

## Major releases (2.0.0)

Required for:
- Any breaking wire-format change (renames, removals, required fields)
- Any semantic redefinition of an existing operation or field
- Any change that alters outcomes for previously valid v1.x patches/documents
- Any change that intentionally invalidates previously valid v1.x inputs

## Rule of thumb

If a v1.0.2-compliant consumer could process the new artifact and obtain the same result
(or safely ignore the extension), it can be 1.1.x. Otherwise it MUST be 2.0.
