# 013 — `serde_json::Value` for the cell value setter

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

`cell.value = <JS value>` crosses the FFI boundary as a `serde_json::Value`. Rust branches
on the JSON shape to pick the `CellValue` variant.

## Context

napi v3's `serde-json` feature auto-converts JS primitives to Rust. A setter that takes a
union — number, string, boolean, `null`, `Date`, or a `{ formula, number }` object — has no
single Rust parameter type that covers it. `serde_json::Value` does.

## Alternatives considered

- **A tagged union object with an explicit `type` field.** Rejected — breaks the
  ExcelJS-compatible setter, where users assign a bare `42` or `"text"`.
- **Separate setters per variant.** Rejected — same compatibility problem.
- **Serialize in JS and parse in Rust by hand.** Rejected — the napi `serde-json` feature
  already does this.

## Consequences

A bare JS number, string, boolean, or `null` dispatches to the obvious variant. An object
with a `formula` key is the cached-formula form. The dispatch table lives in one place and
is the same pattern reused for styles (ADR-026).

## Sources

- `Cargo.toml:10-16` — napi v3 with the `serde-json` feature.
- `src/model/cell.rs` — the setter dispatch.
- `docs/architecture-review.md` — P0-1 and the §6.2 setter dispatch description.
- Rationale otherwise recovered from the v1.0.0 specification's one-line table entry.
