# 006 — Node.js 20+ (NAPI 9)

**Status:** current (corrected — see *Correction*)
**Recorded at:** v0.1.0 (June 2026)

## Decision

Target modern Node.js as the runtime baseline.

## Context

Node 18 reached end-of-life during v0.1.0 development, and a native addon is consumed by
exactly the projects most likely to be on a current LTS.

## Alternatives considered

- **Support Node 18.** Rejected as EOL.
- **Pin an old NAPI version for reach.** Rejected — napi-rs v3 is the current toolchain.

## Correction (added at v2.9.1)

**The original figures in this decision are not backed by anything in the repository.**

- `package.json` declares **no `engines` field at all**, so no Node.js version constraint
  is actually published to consumers.
- `Cargo.toml:13` enables napi-rs's **`napi5`** feature. That is a NAPI 5 *compatibility
  floor* — it makes the addon load on older NAPI hosts — not a NAPI 9 baseline as the
  original title claimed.

The decision's intent (target modern Node, use the current napi-rs toolchain) is sound and
is carried by ADR-014. The specific numbers were aspirational and never became
configuration. This file records the discrepancy rather than silently restating it.

## Sources

- `Cargo.toml:10,13,17` — napi v3 with the `napi5` feature.
- Absence of `engines` in `package.json` verified at v2.9.1.
- OpenSpec change `reconstruct-design-record`, task 2.2.
