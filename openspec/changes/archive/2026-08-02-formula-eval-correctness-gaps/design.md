## Context

The `formula-eval` feature was implemented in PR #52 with a 40-test suite
that passes. Five correctness gaps and five clippy violations remain —
all in `src/formula/bridge.rs`, `src/model/worksheet.rs`,
`src/model/cell.rs`, and `src/writer/xlsx.rs`.

## Goals / Non-Goals

**Goals:**

- Date cells treated as serial numbers in arithmetic and aggregation
- `recalculate()` isolates per-cell parse errors (no batch abort)
- `MIN()`/`MAX()` with zero numeric args return `0` (match Excel)
- Clippy clean under `#![deny(clippy::all)]`

**Non-Goals:**

- Exposing `recalculate()` to JS via `#[napi]` (Rust-only by design)
- Adding new formula functions beyond the existing set
- Extending the parser grammar (parse errors are caught, not fixed)

## Decisions

### Date handling via serial extraction

Add `Value::Date(d) => Ok(d.serial)` to `as_f64()` and equivalent to
`collect_numbers()` (both scalar and array arms). `is_truthy()` already
handles dates; this brings the other two functions to parity. No need to
convert dates to `Value::Number` at the boundary — preserving the `Date`
variant avoids losing date type information for round-trip fidelity.

### Per-cell error isolation in recalculate

Catch `Err` from `FormulaEvaluator::evaluate()` and convert to
`Value::Error(CellError::Value)`. This mirrors the existing per-cell
pattern already in `eval_cell_ref()` (bridge.rs ~L298) where parse errors
are caught and returned as `Outcome::Error(CellError::Value)`. Using
`?` at the recalc level was the inconsistency.

### MIN/MAX empty-args → 0

`fn_min_max` already has an `is_min` flag. When `nums.is_empty()`,
return `Value::Number(0.0)`. `fn_average` keeps `#DIV/0!` (correct).

### write_cell_xml: if → else if

Change standalone `if` to `else if` in the `Formula` arm. Currently
unreachable (value_to_cell_value never sets overlapping fields) but
defensive.

## Risks / Trade-offs

- [New date tests] Risk: `ExcelDate::serial` semantics may drift from
  upstream xlstream-core. Mitigation: tests assert known serial values.
- [Recalc error isolation] Risk: Parse errors cached as `#VALUE!` silently
  mask formula typos. Mitigation: Excel does same — visible to user as cell
  error, not crash. Matches issue #51's "error value in-cell" approach.
- [Clippy fixes] Risk: Zero. Mechanical pattern already used elsewhere.

## Open Questions

None — all decisions resolve against existing codebase patterns.
