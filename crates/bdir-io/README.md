# bdir-io

`bdir-io` is the **single supported public Rust entrypoint** for the BDIR Patch Protocol.

It provides:
- protocol wire types (Edit Packet v1, Patch v1)
- deterministic JSON canonicalization
- hashing utilities (content hashes, cache keys)
- patch validation / application helpers

## Public API stability

External consumers SHOULD import exclusively from:

```rust
use bdir_io::prelude::*;
```

Anything not re-exported through `bdir_io::prelude` is considered **internal** and may change without notice.

## Non-goals

This crate intentionally contains **no** HTML extraction, crawling, or AI logic.
Those belong in higher layers.
