## Patch Application Conformance

The behavior of `apply_patch_against_edit_packet` is locked in by
golden tests and a conformance matrix under:

- `crates/bdir-patch/tests/apply_editpacket_golden.rs`
- `crates/bdir-patch/tests/conformance_matrix.rs`

Changes to these tests are considered protocol-level changes and
must be reviewed accordingly.
