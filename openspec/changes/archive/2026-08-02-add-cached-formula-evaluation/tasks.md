# Tasks: Cached Formula Evaluation via xlstream-parse

Ordered by dependency. Each task verifiable against `specs/formula-eval/spec.md`
(see `design.md` for architecture).

## 1. Dependency & build setup

- [x] 1.1 Add `xlstream-parse` and `xlstream-core` optional deps + `formula-eval` Cargo feature to `Cargo.toml`
      (already done in previous session; off by default, default build unaffected)
- [x] 1.2 Validate build weight: confirm default build (without `formula-eval`) does not pull in xlstream crates;
      confirm `formula-eval` enables deps without Arrow/rayon.

## 2. Evaluation bridge

- [x] 2.1 Create `src/formula/mod.rs` — module root, `Scalar` type alias, `#[cfg(feature = "formula-eval")]` gate.
- [x] 2.2 Create `src/formula/bridge.rs` — `FormulaEvaluator` struct with `parse()`, `NodeRef` traversal,
      `NodeView` match for all 15+ variants (Number, Bool, Text, Error, CellRef, RangeRef, BinaryOp, UnaryOp, Function, Array).
- [x] 2.3 Implement cell reference resolution (`eval_cell_ref`) — `Worksheet::get_cell_by_rc`,
      `CellValue` → `Value` conversion, recursive Formula-cell evaluation with cycle detection.
- [x] 2.4 Implement range reference resolution (`eval_range_ref`) — 2D grid with whole-column/row support.
- [x] 2.5 Implement binary/unary operator dispatch (`apply_binary_op`, `apply_unary_op`)
      — arithmetic + comparison + concatenation with sticky error propagation + type coercion.
- [x] 2.6 Implement built-in function table (`call_function`) — SUM, AVERAGE, MIN, MAX, COUNT, COUNTA,
      IF, AND, OR, NOT, ABS, ROUND, CONCAT, LEFT, RIGHT, MID, LEN, IFERROR (~20 functions).
- [x] 2.7 Implement error type conversions (`cell_error_to_string`, `parse_error_string`,
      `cell_value_to_value`, `value_to_cell_value`, `normalize_formula`).

## 3. Entry points & JS API

- [x] 3.1 Add Rust `FormulaEvaluator::evaluate(formula, row, col) -> Result<Option<Scalar>, ExcelrsError>`.
- [x] 3.2 Add `Worksheet::recalculate(&self) -> Result<(), ExcelrsError>` — iterates formula cells,
      evaluates each, caches result on `CellValue`.
- [x] 3.3 Add JS `Cell.cachedValue` napi read-only getter — returns computed scalar or `null`
      (spec §4 scenario). Gated behind `#[cfg(feature = "formula-eval")]`.
- [x] 3.4 Confirm `Cell.formula` remains read-only (no setter) — ExcelJS parity.

## 4. Cached value & write path

- [x] 4.1 On `Worksheet::recalculate`, store evaluated scalar on `CellValue` cached fields
      (`number`/`string`/`boolean`/`error_value`) so writer emits
      `<f>{formula}</f><v>{computed}</v>` (`writer/xlsx.rs:1845-1850`).
- [x] 4.2 Streaming-path limitation guard: streaming reader keeps formula strings only;
      `cachedValue` returns `null` for streaming cells (spec §5).

## 5. Testing

- [x] 5.1 Test arithmetic + precedence: `=1+2*3` → `7`; `=1/0` → `#DIV/0!`.
- [x] 5.2 Test cell reference resolution: `=A1` returns A1's cached value.
- [x] 5.3 Test cross-sheet reference: `'Sheet 2'!A1` resolves by name.
- [x] 5.4 Test function dispatch: `=SUM(A1:A3)` returns correct sum;
      `=IF(A1>0,"yes","no")` returns `"yes"`.
- [x] 5.5 Test circular reference: `=A1` where A1 contains `=A1` returns `#REF!`.
- [x] 5.6 Test write path: evaluated cell writes `<f>1+2</f><v>3</v>` to XLSX output.
- [x] 5.7 Build/test without `formula-eval`: evaluation API absent from public surface.

## 6. Specs & docs

- [x] 6.1 Supersede Non-Goal text in `docs/spec.md` §10 (evaluation now in-scope via feature).
- [x] 6.2 Update `ROADMAP.md` formula-evaluation status (gated behind `formula-eval`).
- [x] 6.3 Add CHANGELOG entry keyed to the `formula-eval` feature.
- [x] 6.4 Archive this change.
