# 020 — `#[napi(constructor)]` on `new()` methods

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

Annotate every `new()` method with `#[napi(constructor)]`. A bare `#[napi]` is not
sufficient in napi v3.

## Context

**Found in the pre-implementation review (P1-3).** napi v3 requires the attribute
explicitly; without it the function is not registered as a JS constructor. This is a
compile-and-run discovery rather than a judgment call, which is why it got its own decision
number.

## Alternatives considered

None. There is no alternative; the attribute is required.

## Applies to

`Workbook`, `Worksheet`, `Row`, and `Cell` constructors.

## Sources

- `docs/architecture-review.md` — P1-3, "Missing `#[napi(constructor)]` annotation", ✅
  RESOLVED.
- `src/model/*.rs`, `src/workbook.rs` — the annotated constructors.
- `Cargo.toml:10,17` — napi v3 (see ADR-014).
- Rationale otherwise recovered from the v1.0.0 specification's one-line table entry.
