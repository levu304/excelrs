# 011 — Flat `#[napi(object)]` struct for CellValue

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

Model `CellValue` as a single flat `#[napi(object)]` struct with a `value_type: String`
discriminant and an `Option<T>` field per variant. Not a Rust enum.

## Context

**This was a P0 blocker in the pre-implementation architecture review.** A Rust enum with
variant data cannot cross the NAPI boundary: napi-rs supports only C-style enums via
`#[napi(string_enum)]`, which cannot carry payloads. A design that assumed otherwise would
have failed at the Rust level and forced re-architecture mid-implementation.

The discriminant-plus-`Option` shape is the standard workaround: every variant's field is
present but only one is meaningful, and `value_type` says which.

## Alternatives considered

- **`enum CellValue { Null, Number(f64), String(String), ... }`** — rejected. Does not
  compile across the FFI boundary with variant data. This was the original design and the
  review's first P0 finding.
- **`#[napi(string_enum)]` with no payloads.** Rejected — a cell value must carry a
  number, a string, or a rich-text run list.
- **Serialize to JSON and pass a string.** Rejected — allocates and parses on every cell
  access, on the hottest path in the library.
- **One JS class with getters per variant.** Rejected — allocates a wrapper object per cell.

## Consequences

Every variant read and write must branch on `value_type`. This is centralized rather than
scattered: the writer routes every `value_type` transition through a single
`mark_formula`-style discriminant write (change #61), so the discriminant is set in one
place instead of at each call site.

## Sources

- `docs/architecture-review.md` — P0-1, "CellValue as Rust enum → Flat `#[napi(object)]`
  struct", verdict ✅ RESOLVED.
- `docs/architecture-review-pass2.md` §1 — verified the fix, noting the
  `serde_json::Value::Number` `as_f64()` edge case for i64 values not representable as f64.
- `src/model/cell.rs` — the shipped struct.
- Rationale otherwise recovered from the v1.0.0 specification's one-line table entry.
