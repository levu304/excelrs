# 012 — Hand-maintained JS glue for method overloading

**Status:** current (size corrected — see *Correction*)
**Recorded at:** v0.1.0 (June 2026)

## Decision

Express overloaded methods as separate Rust methods plus a thin hand-maintained JavaScript
glue layer that dispatches on argument shape.

## Context

**This was the second P0 blocker in the pre-implementation review.** A single
`getCell(address: string)` and `getCell(row, col)` cannot be one `#[napi]` function —
napi-rs does not support method overloading. The review's finding was that the spec did not
acknowledge this.

## Alternatives considered

- **Rust-side variadic dispatch.** Rejected — napi-rs has no variadic `#[napi]` functions.
- **Distinct method names only** (`getCellByAddress` / `getCellByRC`). Rejected — breaks
  the ExcelJS-compatible API, which is the product.
- **A code generator emitting the glue.** Rejected as a build dependency for ~20 lines of
  dispatch. Revisit only if the glue grows substantially.

## Correction (added at v2.9.1)

**The stated size of this decision is badly stale.** It was written as a "~20-line glue
file" and later as a "~40-line CommonJS file". The actual `index.js` is **621 lines**.

The *decision* is sound and the glue works — `index.js:614-616` dispatches the `getCell`
overloads exactly as described. But the file has grown well past the size at which
"hand-maintain it, don't generate it" remains obviously right, and the ADR-012 rationale
("a code generator is not worth it for 20 lines") no longer supports itself on its own
terms. That is a live question for the next person to add a method, not a resolved one.

## Known sharp edge (from the second-pass review)

`ws.getCell(1)` — a single numeric argument — routes to `getCellByRC(1, undefined)`, where
`undefined` converts to `u32` as `0`. The review recommended an explicit error instead of
silently addressing row 0. Not implemented.

## Sources

- `docs/architecture-review.md` — P0-2, method overloading, ✅ RESOLVED.
- `docs/architecture-review-pass2.md` §1 — P0-2 verification and the `getCell(1)` caveat.
- `index.js:614-616` — the live dispatch; file measured at 621 lines.
- `scripts/apply-glue.cjs` — the build pipe that installs the glue.
- OpenSpec change `reconstruct-design-record`, task 2.2.
