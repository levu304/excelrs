# 005 — Dual MIT / Apache-2.0 license

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

License under MIT OR Apache-2.0, at the user's choice.

## Context

This is the Rust ecosystem's standard convention. It maximizes adoption: corporate legal
review generally clears Apache-2.0, while MIT's brevity suits small projects, and the dual
form lets each consumer pick.

## Alternatives considered

- **MIT only.** Rejected — Apache-2.0's explicit patent grant matters to some adopters.
- **Apache-2.0 only.** Rejected — MIT's simplicity suits the project's audience.
- **AGPL.** Rejected — would prevent most commercial use of a library whose purpose is
  drop-in replacement inside other products.

## Sources

- `LICENSE-MIT`, `LICENSE-APACHE`, `LICENSE` (pointer).
- `package.json:7` — `"license": "MIT OR Apache-2.0"`.
- Rationale recovered from the v1.0.0 specification's one-line table entry.

> **Numbering note.** This file is `005` in the v1.0.0 specification's ADR table. An
> unrelated decision was previously extracted under the number
> `005-streaming-write-buffering` from a different numbering stream; that collision is
> resolved in [ADR-028](028-streaming-write-buffering.md).
