# 021 — `#[napi(setter)]`, not `#[napi(set)]`

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

Use the `#[napi(setter)]` attribute for JS property setters. The spelling is `setter`, not
`set`.

## Context

Like ADR-020, this is a v3 toolchain detail rather than a judgment call, recorded separately
because it is a distinct compile-time discovery with no alternative.

## Applies to

Every JS-visible property setter: `cell.value`, `cell.style`, `row.height`, worksheet
properties.

## Sources

- `src/model/*.rs` — the annotated setters.
- `Cargo.toml:10,17` — napi v3 (see ADR-014).
- Rationale recovered from the v1.0.0 specification's one-line table entry.
