# Proposal: Formula Evaluation via formularizer-eval (additive)

## Why

excelrs preserves formula strings across read/write but never evaluates them: formulas
are opaque `<f>` payloads, cached `<v>` is retained only on the whole-workbook reader
(`writer/xlsx.rs:1845-1850`) and deliberately dropped on the streaming path
(`stream.rs:778-781`). This matches ExcelJS parity — `docs/spec.md` §10 and `ROADMAP.md`
both list formula evaluation as a **Non-Goal**, deferred v3+.

A real resolver-injection engine already exists and de-risks the escape hatch ADR-9
reserves: `formualizer-eval` (latest 0.7.x, MIT/Apache-2.0, 400+ Excel functions) is
architected so the *consumer* owns the data model and supplies cell/range resolution
through a public trait (`ReferenceResolver` / `RangeResolver` / `TableResolver` /
`EvaluationContext`; `Range` is a 2-method `get`+`dimensions` contract). excelrs already
exposes that exact seam — `Worksheet::get_cell_by_address` (`worksheet.rs:155`),
read-only `cell.formula()` (`cell.rs:449`), and `CellValue` leaf scalars (`cell.rs:411`).
Evaluation can therefore be added **additively** behind a Cargo feature, with no rewrite of
`WorkbookInner`/`Cell`. This change turns on the ADR-9 "integration" branch.

## What Changes

- Add optional dep `formularizer-eval` behind a new, off-by-default Cargo feature
  `formula-eval`.
- Add a bridge module implementing `EvaluationContext` over excelrs's own
  `Worksheet`/`Cell`/`CellValue` (resolver returns cached scalars; formula string via
  `cell.formula()`). **No change** to the `WorkbookInner`/`Cell` model.
- Add eval entry points: Rust `Cell::evaluate(...)` / `Worksheet::recalculate(...)`;
  JS mirror `Cell.cachedValue` getter. `cell.formula` stays read-only (ExcelJS parity —
  no `cell.formula =` setter is introduced).
- Populate the cached value so `<v>` is emitted for evaluated formula cells
  (`writer/xlsx.rs:1845-1850` already renders `<v>` once a computed value is present;
  only the value source changes).
- Supersede Non-Goal §10 (`docs/spec.md`) + `ROADMAP.md` formula-evaluation deferral:
  evaluation is now in-scope, via integration, gated behind the `formula-eval` feature.
  (Spec-text change only — every public API addition is additive.)

**Not in scope** (documented gaps): dynamic-array spill ranges / `#SPILL!`, and
whole-workbook dependency-driven recalc ordering beyond per-cell evaluation. These are
explicitly deferred; the bridge is built to make them pluggable later.

## Capabilities

### New

- `formula-eval`: cell/worksheet formula evaluation via an external
  resolver-injection engine, cached-value materialization on write, and the streaming-path
  limitation (cached `<v>` still not retained on the streaming reader by default).

### Modified

(empty — no live spec in `openspec/specs/` currently governs formula behavior; formula
preservation is today implicit / a Non-Goal, not a committed requirement. The
Non-Goal text supersession is tracked under Impact.)

## Impact

- **Rust**: new `feature = "formula-eval"`; new `src/formula/` bridge + `Cell::evaluate`
  / `Worksheet::recalculate` methods; new napi getter `Cell.cachedValue`.
- **JS**: new `cell.cachedValue` getter; existing `cell.formula` unchanged (read-only).
- **Writer**: cached `<v>` populated for evaluated formula cells (no XML format change;
  `<f>` already written at `writer/xlsx.rs:1817`, `<v>` at `1845-1850`).
- **Reader streaming path**: cached `<v>` still not retained by default — streaming eval
  is N/A until a future opt-in (documented gap, not regressed).
- **Dependency**: adds `formularizer-eval` (+ transitively Arrow/rayon on the enabled
  path) only when the `formula-eval` feature is active.
- **Specs/docs**: supersede Non-Goal text in `docs/spec.md` §10 + `ROADMAP.md`.
- **Backwards compatibility**: none — public API additions are additive; the feature is
  opt-in and off by default. No **BREAKING** changes.
