# Implementation Status vs RFC

This document describes the **current implementation status** of the BDIR Patch Protocol
as implemented by reference engines and tooling.

> ⚠️ **Non-normative**
>
> This document is informational only.
> The authoritative definition of the protocol is **RFC-0001**.

---

## Why this document exists

The BDIR Patch Protocol RFC defines **what compliant implementations MUST or SHOULD do**.

However, in practice:

- Not all RFC features may be implemented yet
- Some features may be optional or experimental
- Some behaviors may be planned but not shipped

This document exists to prevent ambiguity between **specification guarantees** and
**implementation reality**.

---

## Scope

This document applies to:

- Reference engines
- Example tooling
- Patch validation and application pipelines

It does **not** override, amend, or reinterpret the RFC.

---

## Feature status matrix

| Feature | RFC Status | Implementation Status | Notes |
|------|-----------|-----------------------|------|
| Block-level patch operations | REQUIRED | ✅ Implemented | `replace`, `delete`, `insert_after`, `suggest` |
| Page-level content hash binding | REQUIRED | ✅ Implemented (configurable) | Patch `h` binding validated; enforcement may be toggled via options in some integration layers |
| Deterministic patch validation | REQUIRED | ✅ Implemented | All-or-nothing semantics |
| kind_code importance guidance | SHOULD | ✅ Implemented | Prompt-level only |
| Caching guidance | SHOULD | ⚠️ External | Engine-agnostic |
| Telemetry fields | SHOULD | ❌ Not implemented | Planned |

---

## Known gaps

- Page-level hash enforcement is not mandatory by default
- Telemetry fields are not standardized across engines
- No formal conformance test suite exists yet

---

## Roadmap alignment

Implementation gaps are tracked via GitHub issues using labels such as:

- `safety`
- `determinism`
- `docs`

See the issue tracker for current status and milestones.

---

## Updating this document

When implementation behavior changes:

1. Update this file
2. Reference the relevant issue or pull request
3. **Do not** modify the RFC unless protocol semantics change