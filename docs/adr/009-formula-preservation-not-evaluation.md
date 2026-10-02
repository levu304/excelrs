# 009 — Formula preservation, not evaluation

**Status:** superseded
**Recorded at:** v0.1.0 (June 2026)
**Superseded by:** partial implementation of the "separate product" — see below

## Decision

Preserve formula strings for round-trip fidelity. Do **not** evaluate formulas. A full
evaluator is a separate product.

## Context

ExcelJS evaluates nothing. Preserving the formula string is sufficient for parity, and
round-trip fidelity is the harder correctness property. A 500-function evaluator is a
multi-year project with its own correctness surface, and building it inside a port would
have jeopardized the port.

## Alternatives considered

- **Build an evaluator.** Rejected as a separate product.
- **Drop formulas.** Rejected — they are part of the file format and of ExcelJS behavior.

## Why superseded

**The "separate product" was partly built.** `Workbook.recalculate()` and
`Worksheet.recalculate()` shipped in **v2.9.0** (issue #62), caching computed values back
into cells and round-tripping them as `<f>..</f><v>..</v>`.

The engine is **opt-in behind the `formula-eval` Cargo feature** and not compiled into
default builds. `Cargo.toml` carries `xlstream-parse` and `xlstream-core` for it;
`formularizer-eval` was yanked from crates.io, which is documented in
`docs/formula-engine-research.md`. Coverage is roughly 20 functions against Excel's 500+.

So the decision's *shape* held — evaluation stayed out of the default build and stayed a
separable concern — but its premise, that no evaluator would be built, no longer describes
the code.

## Consequence: a live citation-drift hazard

`ROADMAP.md` cited this decision as the governing policy for full formula evaluation
("a standalone interpreter is a separate product (ADR-9)") after v2.9.0 shipped
`recalculate()`. That is the same failure class as the `exceljs-parity` drift corrected in
#66: a document anchoring itself to a decision that no longer holds.

Correcting that citation is part of the change that superseded this decision. Whether
`formula-eval` should be promoted out of its opt-in feature is a **separate decision, not
made here**.

## Sources

- `Cargo.toml` — `formula-eval` feature, `xlstream-parse`/`xlstream-core` optional deps, and
  a `ponytail:` comment recording the crates.io yank.
- `CHANGELOG.md` — v2.9.0 `recalculate()` entry, closing #62.
- `docs/formula-engine-research.md` — engine selection and the yank.
- `docs/specs/formula-eval/` — the shipped capability.
- OpenSpec change `reconstruct-design-record`, design D5 and D6.
