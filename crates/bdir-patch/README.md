## Patch Application Conformance

The behavior of `apply_patch_against_edit_packet` is locked in by
golden tests and a conformance matrix under:

- `crates/bdir-patch/tests/apply_editpacket_golden.rs`
- `crates/bdir-patch/tests/conformance_matrix.rs`

Changes to these tests are considered protocol-level changes and
must be reviewed accordingly.

## Safety: page hash binding

Patch application and validation now require an explicit page-hash binding.

A patch must either:
- include `h` (the page-level hash from the Edit Packet / Document), **or**
- be validated/applied with `ValidateOptions.expected_page_hash` provided out-of-band.

This prevents accidental application of patches to a different page version.

