## Why

PR #52 introduced the `formula-eval` Cargo feature implementing the `formula-eval` spec, but exploration uncovered five correctness gaps and five clippy violations that would either break compilation under `#![deny(clippy::all)]` or produce silently wrong results compared to Excel semantics. These were not caught by the 40-test suite because no tests exercise date arithmetic, parse-error resilience, empty-argument edge cases, or clippy-cleanliness.

## Changes

- **Date serial handling** (behavioral fix): `as_f64()` and `collect_numbers()` in `src/formula/bridge.rs` now handle `Value::Date(d)` by extracting `d.serial` (f64). Previously `Value::Date` fell through to the catch-all arm, causing `=A1+1` where A1 is a date to return `#VALUE!` and `SUM(date_range)` to silently exclude date cells.
- **Parse-error resilience** (design fix): `Worksheet::recalculate()` in `src/model/worksheet.rs` now catches `Err` from `FormulaEvaluator::evaluate()` per-cell and caches `Value::Error(CellError::Value)` instead of propagating `?` and aborting the entire recalculation batch. This matches Excel semantics where a parse error in one cell does not prevent other cells from computing.
- **MIN/MAX empty-args** (behavioral fix): `fn_min_max()` in `src/formula/bridge.rs` now returns `0` when no numeric arguments are present, matching Excel behavior (both `MIN()` and `MAX()` with zero numeric args return `0`; only `AVERAGE()` returns `#DIV/0!`).
- **Clippy compliance** (5 fixes, non-behavioral): `arith_div`/`arith_mod` redundant guards, `fn_round`/`fn_iferror` `len() < 1` → `is_empty()`, `cached_value()` `field_reassign_with_default` → `..Default::default()`. These are required because the crate declares `#![deny(clippy::all)]`.
- **write_cell_xml else-if** (code smell fix): The `error_value` branch in `src/writer/xlsx.rs` `write_cell_xml` `Formula` arm is changed from a standalone `if` to `else if` for defensive consistency. Currently unreachable (value_to_cell_value never sets overlapping fields), but prevents latent XML corruption if field semantics change.

## Capabilities

### Modified Capabilities

- `formula-eval`: Requirements added/updated for (a) date cells participate in numeric arithmetic as serial numbers, (b) `recalculate()` isolates per-cell parse errors without aborting the batch, (c) `MIN()`/`MAX()` with zero numeric args return `0`, (d) clippy lint compliance under `#![deny(clippy::all)]`.

## Impact

- `src/formula/bridge.rs` — `as_f64`, `collect_numbers`, `fn_min_max`, `arith_div`, `arith_mod`, `fn_round`, `fn_iferror`
- `src/formula/tests.rs` — new tests for date arithmetic, empty-arg MIN/MAX, parse-error resilience
- `src/model/worksheet.rs` — `recalculate` error boundary
- `src/model/cell.rs` — `cached_value()` clippy fix
- `src/writer/xlsx.rs` — `write_cell_xml` else-if fix
- Test suite grows by ~6 tests
