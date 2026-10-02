# 019 — Clone-on-read mutation semantics

**Status:** superseded (resolved in v0.4.0)
**Recorded at:** v0.1.0 (June 2026)
**Superseded by:** interior mutability via `Arc<Mutex<CellInner>>`, v0.4.0

## Decision

(v0.1) `ws.getCell('A1').value = 42` mutates a detached copy. The assignment is silently
discarded.

## Context

The straightforward Rust implementation returns a `Cell` by value. Assigning to a field of
a temporary compiles, appears to work, and does nothing. This is the worst failure shape
available: silent, and it type-checks.

## Alternatives considered

- **Accept it and document it.** Rejected — silently discarded writes are unacceptable in
  a library whose purpose is correctness.
- **Return a reference.** Rejected — not expressible across the NAPI boundary.

## Resolution

`Cell` now holds `Arc<Mutex<CellInner>>`, so the value returned by `getCell` shares interior
state with the worksheet and `ws.getCell('A1').value = 42` persists. Shipped in v0.4.0.

The decision is recorded rather than deleted because the failure mode it documents — a
Rust-shaped API that silently no-ops across an FFI boundary — is the kind of thing that
reappears when new types cross that boundary.

## Sources

- OpenSpec archived change `2026-07-09-cell-interior-mutability`, decision D1: "Wrap `Cell`
  data in `Arc<Mutex<CellInner>>`".
- `CHANGELOG.md` — v0.4.0.
- `src/model/cell.rs`.
- This was the one in-table decision already annotated as resolved before reconstruction.
