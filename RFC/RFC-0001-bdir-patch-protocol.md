# RFC-0001: BDIR Patch Protocol

**Status:** Draft  
**Intended Status:** Informational  
**Version:** 1.0.0  
**Last Updated:** 2026-01-16  
**Authors:** C.A.G. van de Luitgaarden

---

## Abstract

This document specifies the **BDIR Patch Protocol**, a stateless protocol for AI-assisted content review. The protocol constrains AI systems to analyze complete documents while producing block-scoped patch instructions rather than rewritten content. It is designed to support deterministic validation, auditability, and human-in-the-loop workflows in regulated or large-scale content environments.

---

## Status of This Memo

This document is not an Internet Standards Track specification; it is published for informational purposes. Distribution of this memo is unlimited.

> **Non-normative note**
>
> This document defines protocol requirements.
> Current implementation status is tracked separately in
> `docs/implementation-status.md`.

---

## Copyright Notice

Copyright © 2026 the authors.  
All rights reserved.

---

## 1. Introduction

Recent advances in large language models (LLMs) have enabled automated analysis and modification of textual content. Most existing approaches rely on direct rewriting of documents, which introduces risks related to unintended semantic changes, lack of auditability, and unsuitability for regulated or safety-critical environments.

The BDIR Patch Protocol defines a different interaction model. Under this protocol, an AI system analyzes a complete document but is restricted to producing a set of deterministic, block-level patch instructions. These instructions can be validated, reviewed, and applied by downstream systems under strict safety constraints.

The protocol is intentionally stateless, AI-agnostic, and independent of any specific content management system. It is designed to enable automation while preserving human oversight and traceability.

---

## 2. Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** in this document are to be interpreted as described in BCP 14 (RFC 2119 and RFC 8174).

**BDIR (Block-based Document Intermediate Representation)**  
A canonical representation of a document as an ordered sequence of semantic blocks.

**Edit Packet**  
A minimized derivative of BDIR used as input to an AI system.

**Patch**  
A set of instructions describing proposed modifications to BDIR content.

**Block**  
An atomic unit of content identified by a stable identifier.

---

## 3. Goals and Non-Goals

### 3.1 Goals

The BDIR Patch Protocol is designed to:

- Enable AI systems to review complete documents while producing scoped, reviewable changes
- Reduce the risk of unintended semantic drift
- Support deterministic validation and application of proposed changes
- Minimize operational cost, including token usage
- Remain compatible with human editorial workflows

### 3.2 Non-Goals

The protocol does not aim to:

- Enable real-time collaborative editing
- Automatically resolve conflicting edits
- Replace human editorial judgment
- Define content extraction or segmentation algorithms
- Guarantee correctness of AI-proposed changes

---

## 4. Protocol Model

An AI system participating in this protocol **MUST** be treated as an untrusted proposer. All outputs produced by the AI **MUST** be validated prior to application. No assumption is made regarding the correctness, intent, or reliability of the AI system.

The protocol separates **analysis** (performed by the AI) from **application** (performed by a downstream system under validation).

---

## 5. BDIR Overview (Informative)

BDIR represents a document as an ordered list of semantic blocks. Each block has:

- a stable identifier
- a semantic classification
- canonical text content
- a content hash

BDIR is format-agnostic. While Markdown is commonly used as the canonical text encoding, the protocol does not require a specific markup language.

---

## 6. Edit Packet

### 6.1 Purpose

The Edit Packet provides sufficient document context for AI-assisted review while minimizing token usage. It is derived from BDIR and is not intended to be a complete or lossless serialization.

### 6.2 Wire Format

The Edit Packet is encoded as a JSON object:

```json
{
  "v": 1,
  "tid": "string (optional)",
  "h": "contentHash",
  "ha": "hashAlgorithm",
  "b": [
    ["blockId", kindCode, "textHash", "text"]
  ]
}
```

### 6.3 Field Semantics

- `v`  
  Protocol version. This document defines version `1`.

- `tid`  
  Optional trace identifier. This value MAY be used by implementations to associate the packet with external metadata such as a URL or retrieval timestamp.

- `h`  
  Page-level content hash. This value MUST match the hash of the BDIR content used to generate the packet.

- `ha`  
  Hash algorithm used for block-level `textHash` values.

- `b`  
  An ordered list of block tuples representing the document content.

### 6.4 Block Tuple

Each block tuple has the following structure:

```
[blockId, kindCode, textHash, text]
```

- `blockId` MUST be stable across extractions
- `textHash` MUST correspond to the provided `text`
- `text` MUST represent the canonical block content

---

## 7. kindCode Semantics

The `kindCode` field communicates block importance rather than presentation details.

### 7.1 Importance Ranges

| Range | Semantics |
|------|-----------|
| 0–19 | Core content or structure |
| 20–39 | Boilerplate or navigation |
| 40–59 | User interface chrome |
| 99 | Unknown |

AI systems SHOULD avoid proposing modifications to lower-importance blocks unless clear errors are present.

---

## 8. Patch Instructions

### 8.1 General Rules

AI systems **MUST** output patch instructions rather than rewritten documents.

### 8.2 Supported Operations

The protocol defines the following operation types:

- `replace`
- `delete`
- `insert_after`
- `suggest`

### 8.3 Validation Requirements

All patch instructions:

- MUST reference an existing `blockId`
- MUST be validated prior to application

For `replace` and `delete` operations:

- An exact `before` substring MUST be provided
- The substring MUST match verbatim within the target block text

Failure of any validation step **MUST** result in rejection of the entire patch.
### 8.4 Canonical operation ordering (Determinism)

Patch `ops` arrays have no semantic ordering requirement in this RFC; however, implementations **SHOULD** canonicalize operation ordering before storing, hashing, caching, diffing, or displaying patches. Canonical ordering reduces review noise and enables deterministic cache keys.

When canonicalizing, implementations **SHOULD** sort operations by:

1. `blockId` ascending (lexicographic), or by the block's document order when the source Edit Packet is available
2. Operation type in this order: `delete`, `replace`, `insert_after`, `suggest`
3. Operation-specific fields (`before`, `after`, `content`, `message`, `occurrence`)

If any ties remain, implementations **SHOULD** apply a deterministic tie-breaker (e.g., original index).


---

## 9. Patch Application

A patch MUST only be applied if all of the following conditions are met:

1. The page-level content hash matches
2. All referenced blocks exist
3. All `before` substrings match exactly

Implementations MUST treat patch application as an all-or-nothing operation.

---

## 10. Caching and Deduplication

Implementations SHOULD cache patch results using a deterministic key derived from:

- the AI model identifier
- the prompt or instruction version
- the patch schema version
- a hash of the Edit Packet

Identical inputs SHOULD result in identical patches.

---

## 11. Telemetry

Implementations SHOULD record operational telemetry, including:

- input token count
- output token count
- total token count
- cache hit or miss status

Telemetry data MAY be used for auditing, cost analysis, and operational monitoring.

---

## 12. Security Considerations

The protocol enforces safety through structural constraints rather than behavioral assumptions. Implementations MUST validate all patches prior to application.

This protocol does not prevent AI systems from proposing incorrect or undesirable changes; instead, it ensures that such changes are detectable, reviewable, and rejectable before application.

---

## 13. Related Work

The BDIR Patch Protocol is related to, but distinct from, existing mechanisms such as JSON Patch (RFC 6902), operational transformation systems, editorial suggestion workflows, and AI text rewriting APIs. These systems either lack semantic awareness, deterministic validation, or suitability for automated and regulated environments.

---

## 14. Call for Comments

Feedback is invited from implementers, content platform maintainers, and AI system integrators. In particular, feedback is requested on:

- the sufficiency of the Edit Packet format
- validation and safety mechanisms
- interoperability across content extraction pipelines
- operational experience at scale

Such feedback will inform future revisions of this document.

---

## 15. References

### 15.1 Normative References

- RFC 2119: *Key words for use in RFCs to Indicate Requirement Levels*
- RFC 8174: *Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words*

### 15.2 Informative References

- RFC 6902: *JavaScript Object Notation (JSON) Patch*
