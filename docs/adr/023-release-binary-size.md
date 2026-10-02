# 023 — Release binary size

**Status:** superseded
**Recorded at:** v0.1.0 (June 2026)
**Superseded by:** no fixed size target — see below

## Decision

A release binary of roughly 440 KB (darwin-arm64) is acceptable for a platform-specific
native addon.

## Context

Binary size affects download and install time for every consumer, so it was worth pinning a
budget at the point where the crate was first built.

## Alternatives considered

- **Strip symbols and enable LTO.** Accepted, and still in `Cargo.toml`:
  `[profile.release] lto = true`, `strip = "symbols"`. This is the part of the decision
  that still holds.
- **Size as a hard release gate.** Rejected then, and not adopted later.

## Why superseded

**The figure is stale by roughly 40x.** Measured at v2.9.1:

| Binary | Size |
| --- | --- |
| `excelrs.darwin-arm64.node` | **17.3 MB** |
| `excelrs.linux-x64-musl.node` | 3.2 MB |
| `excelrs.linux-x64-musl.node` | 3.6 MB |

darwin-arm64 carries debug/profiling symbols in the local build and is the outlier; the
stripped musl builds are the fairer comparison, and they are still ~7x the original figure.
The growth tracks the feature surface — streaming, styles, tables, conditional formatting,
images, and formula evaluation all landed after this decision.

**The number is dropped rather than restated.** A pinned size is a snapshot that goes stale
exactly the way ADR-010's target list did, and no CI gate enforces it, so an updated figure
would be a number nobody maintains. The LTO and strip settings are the durable, enforced
part and remain in `Cargo.toml`.

## Sources

- Binary sizes measured directly at v2.9.1.
- `Cargo.toml:35-37` — `[profile.release] lto = true`, `strip = "symbols"`.
- Rationale otherwise recovered from the v1.0.0 specification's one-line table entry.
