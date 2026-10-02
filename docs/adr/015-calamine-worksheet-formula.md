# 015 — Read formulas via calamine's `worksheet_formula()`

**Status:** current (scope note added)
**Recorded at:** v0.1.0 (June 2026)

## Decision

calamine stores formulas in a **separate API** from cell data. The reader must call both
`rows()` and `worksheet_formula()` and merge the results by cell address. Iterating only one
silently loses every formula in the workbook.

## Context

This was surfaced in the pre-implementation review as part of the P1 finding that
calamine's `Data` enum and excelrs's `CellValue` are not 1:1. `Formula` has no calamine
source in `Data` at all, so a reader written against `rows()` alone produces a workbook
where every formula has become its cached value — a silent, plausible-looking corruption.

## Alternatives considered

- **Iterate `rows()` only.** Rejected — silently drops formulas.
- **Parse formula XML separately from the zip.** Rejected — reimplements what calamine
  already does for the in-memory path.

## Current form

The reader runs three passes, documented at `src/reader/xlsx.rs:2441`:

1. **Values pass** — `rows()` → cell data.
2. **Style pass** — cell `s` index → resolved `Style`.
3. **Formula pass** — `worksheet_formula().used_cells()` → `Cell.formula`.

`worksheet_formula()` is still called at `src/reader/xlsx.rs:2496`, and the split is
restated in the module header at `reader/xlsx.rs:10`.

## Scope note (added at v2.9.1)

This decision covers the **in-memory** read path. The streaming reader does not call
calamine: it maintains its own per-sheet `SharedFormulaTable`
(`src/stream.rs:610-614`) and resolves shared-formula member cells itself, with reference
translation ported from calamine 0.35.0 and unit tests mirroring calamine's behavior so the
two paths can be compared. That path is a separate mechanism and is not covered by this
decision.

## Sources

- `src/reader/xlsx.rs:10,2441,2496` — the three-pass structure and the live call.
- `src/stream.rs:610-614` — the streaming path's own resolution.
- `docs/architecture-review.md` — P1 finding on calamine `Data` ↔ `CellValue` mapping.
- `CHANGELOG.md` — streaming shared-formula read resolution (#25.2, #32).
- OpenSpec change `reconstruct-design-record`, task 2.1.
