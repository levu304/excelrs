# Proposal: Cached Formula Evaluation via xlstream-parse + Custom Evaluator

## Why

excelrs is a pure I/O layer — it reads and writes XLSX files, preserving
formula strings as opaque `<f>` payloads. Cached `<v>` values are retained
only on the whole-workbook reader path (`reader/xlsx.rs`) and deliberately
dropped on the streaming path (`stream.rs:778-781`, `stream.rs:53`). Formula
evaluation has been a documented **Non-Goal** (`docs/spec.md` §10,
`ROADMAP.md`).

The original `formula-eval-integration` change was blocked:
`formularizer-eval` v0.7.x was **yanked from crates.io** (API returns 404,
GitHub `psu3d0/formularizer` 404). The engine that `exceljs` / ExcelJS users
expected as an integration target no longer exists as a published crate.

**Replacement approach:** The parser half of `formularizer-eval` survives as
`xlstream-parse` v0.4.0 on crates.io (re-exports
`formularizer-parse` v2.0.0 + `formularizer-common` v2.0.0 as transitive
deps). The engine's dependency graph (Arrow, rayon) can be avoided by writing
a custom ~450-line evaluator that consumes the `xlstream-parse` AST directly.

exceljs already exposes the data-model seam needed for this:

- `Worksheet::get_cell_by_address` (`worksheet.rs:155`) → resolves cell refs
- `Worksheet::get_cell_by_rc(row, col)` → 1-indexed cell lookup
- `Cell::value_raw()` → returns `CellValue` with `value_type`, `number`,
  `string`, `boolean`, `error_value`, `formula`, `date_serial`
- `Cell::formula()` (napi getter, `cell.rs:449`) → formula string

## Changes

- **Cargo:** add optional deps `xlstream-parse = "0.4"` and
  `xlstream-core = "0.4"` behind a new `formula-eval` feature (off by
  default). No impact on default build graph.
- **Bridge module** `src/formula/bridge.rs`: custom evaluator that walks the
  `xlstream-parse` AST, resolves cell/range references through the excelrs
  model, applies arithmetic + comparison operators with sticky error
  propagation, and dispatches 20 built-in functions (SUM, AVERAGE, MIN, MAX,
  COUNT, COUNTA, IF, AND, OR, NOT, ABS, ROUND, CONCAT, LEFT, RIGHT, MID, LEN,
  IFERROR). ~450 lines, no Arrow/rayon dependency.
- **Entry points:** Rust `FormulaEvaluator` struct + `Worksheet::recalculate`
  method; JS mirror `Cell.cachedValue` napi getter (read-only).
- **Cached value storage:** evaluated scalars written to `CellValue` cached
  fields (`number`/`string`/`boolean`/`error_value`) so the writer emits
  `<f>formula</f><v>computed</v>` via the existing write path
  (`writer/xlsx.rs:1845-1850`).
- **Model untouched:** no changes to `WorkbookInner` or `Cell` internals beyond
  the additive `cachedValue` getter. `Cell.formula` stays read-only
  (ExcelJS parity).

## Capabilities

- `formula-eval` — formula parsing and evaluation with cached value storage

## Non-Goals

- Dynamic-array spill ranges / `#SPILL!` propagation.
- Whole-workbook, dependency-ordered recalculation (per-cell first phase).
- Streaming-path evaluation (cached `<v>` still dropped on streaming reader;
  documented gap, not a regression).
- Replacing formula-preservation behavior on the default build.
