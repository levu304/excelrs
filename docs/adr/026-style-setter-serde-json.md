# 026 — Style setter via `serde_json::Value`

**Status:** current
**Recorded at:** v0.2.0

## Decision

`cell.style = {...}` crosses the FFI boundary as a `serde_json::Value`. One `set_style`
method branches on which keys are present rather than having a method per style sub-type.

## Context

The same napi v3 `serde-json` pattern as the cell value setter (ADR-013). Style is a
composite of five sub-types — font, fill, border, alignment, number format — so the JS
object nests freely and Rust dispatches on object presence.

## Alternatives considered

- **One method per sub-type** (`setFont`, `setFill`, ...). Rejected — the ExcelJS API is a
  single `style` property.
- **A separate type per sub-type with its own setter.** Rejected — the nesting is the
  ergonomic win, and splitting it would break the compatible shape.

## Consequences

The setter must distinguish "absent" from "explicitly null", since `None` and `Some("")`
are different values. The canonical serialization rules that make style dedup deterministic
are specified in `docs/spec.md` §4 (*Canonical style serialization*) and are load-bearing
for ADR-027.

## Sources

- `Cargo.toml:10-16` — napi v3 `serde-json` feature.
- `src/model/style.rs` — the setter dispatch.
- `src/writer/styles.rs:71` — the canonical key used for dedup (ADR-027).
- Rationale recovered from the v1.0.0 specification's one-line table entry.
