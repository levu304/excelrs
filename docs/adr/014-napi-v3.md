# 014 — napi-rs v3 and `@napi-rs/cli` v3

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

Build on napi-rs v3 with `@napi-rs/cli` v3, using the `serde-json` feature.

## Context

**This was P1-1 in the pre-implementation review.** The original spec targeted napi v2. The
v3 scaffold generates crates with `serde-json` support and modern build tooling, and v2 is
no longer the current toolchain.

## Alternatives considered

- **Stay on napi v2.** Rejected — superseded toolchain, and the v3 scaffold is what the
  project was generated from.
- **Stay on v2 but add `serde-json` manually.** Rejected — no reason to diverge from the
  scaffold.

## Consequences

`#[napi(constructor)]` is required explicitly in v3 rather than inferred from a bare
`#[napi]` — see ADR-020. The attribute spelling for setters differs too — see ADR-021.
Both are v3-specific and were recorded as separate decisions because each was a distinct
compile-time discovery.

## Sources

- `Cargo.toml:10,17` — `napi = { version = "3", ... }`, `napi-derive = "3"`.
- `package.json` devDependencies — `@napi-rs/cli: ^3.2.0`.
- `docs/architecture-review.md` — P1-1, napi v2 → v3, ✅ RESOLVED.
- Rationale otherwise recovered from the v1.0.0 specification's one-line table entry.
