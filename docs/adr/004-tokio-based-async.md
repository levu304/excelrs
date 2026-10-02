# 004 — Tokio-based async

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

Build the async surface on Tokio.

## Context

napi-rs is Tokio-native: its `async` and `tokio_rt` features are implemented against a
Tokio runtime. Choosing anything else means fighting the FFI layer's own assumptions.

## Alternatives considered

- **A dedicated thread pool per operation.** Rejected — reinvents what the FFI layer
  already provides, and loses the shared-runtime model napi-rs expects.
- **Synchronous blocking calls behind the FFI boundary.** Rejected — blocks the Node event
  loop on large workbooks, which is the performance problem this project exists to solve.

## Sources

- `Cargo.toml:26` — `tokio = { version = "1", features = ["rt", "rt-multi-thread", "sync"] }`.
- `Cargo.toml:10-16` — napi features `async` and `tokio_rt`.
- Rationale recovered from the v1.0.0 specification's one-line table entry.
