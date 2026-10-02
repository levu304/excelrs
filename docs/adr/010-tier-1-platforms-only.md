# 010 — Tier 1 platforms only

**Status:** superseded
**Recorded at:** v0.1.0 (June 2026)
**Superseded by:** the release build matrix as the source of truth

## Decision

Ship prebuilt binaries for `darwin-arm64`, `linux-x64`, and `win32-x64` only. Together
these cover the large majority of Node.js users.

## Context

Prebuilt binaries are the expensive part of a release: each needs a CI runner, a platform
package, an `optionalDependencies` entry, and a smoke test. Covering the top three first is
the highest value per unit of release infrastructure.

## Alternatives considered

- **Ship source-only first.** Rejected — the entire value proposition is avoiding a
  toolchain on the consumer's machine.
- **Cover every target immediately.** Rejected as disproportionate release cost.

## Why superseded

The matrix grew, and this decision's list is now a **stale inventory** — the exact rot class
this project later wrote a rule against. It is retained as history.

**The target list is no longer recorded in a specification at all.** It lives in
`.github/workflows/release.yml`, which is the source of truth, and
`package.json` `napi.targets` must agree with it. The `platform-targets` capability states
that agreement as an invariant rather than enumerating targets, so a new target no longer
makes a requirement untrue.

Current matrix (6 targets): `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
`x86_64-pc-windows-msvc` — plus the main package, 7 npm packages in total.

## Sources

- `.github/workflows/release.yml` — the matrix, read as authoritative.
- `package.json` `napi.targets` — 6 entries, in agreement at v2.9.1.
- Rationale recovered from the v1.0.0 specification's one-line table entry.
