## 1. Date handling in formula evaluator

- [x] 1.1 Add `Value::Date(d) => Ok(d.serial)` to `as_f64()` in `src/formula/bridge.rs`
- [x] 1.2 Add `Value::Date(d) => nums.push(d.serial)` to scalar arm of `collect_numbers()` in `src/formula/bridge.rs`
- [x] 1.3 Add `Value::Date(d) => nums.push(d.serial)` to array arm of `collect_numbers()` in `src/formula/bridge.rs`
- [x] 1.4 Add tests: `=date_cell+1`, `SUM(date_range)`, `MIN(date_range)` in `src/formula/tests.rs`

## 2. Per-cell parse error isolation in recalculate

- [x] 2.1 Replace `?` propagation in `Worksheet::recalculate` (`src/model/worksheet.rs`) with per-cell `match` that caches `Value::Error(CellError::Value)` on parse failure
- [x] 2.2 Add test: parse error in one cell does not block cached value on next cell

## 3. MIN/MAX empty-args behavior

- [x] 3.1 Modify `fn_min_max` in `src/formula/bridge.rs` to return `Value::Number(0.0)` when `nums.is_empty()`
- [x] 3.2 Add tests: `=MIN()` returns 0, `=MAX()` returns 0

## 4. Clippy compliance (deny-level)

- [x] 4.1 Fix `redundant_guards` in `arith_div` (bridge.rs:552) — `Ok(b) if b == 0.0` → `Ok(0.0)`
- [x] 4.2 Fix `redundant_guards` in `arith_mod` (bridge.rs:562) — same pattern
- [x] 4.3 Fix `len_zero` in `fn_round` (bridge.rs:764) — `args.len() < 1` → `args.is_empty()`
- [x] 4.4 Fix `len_zero` in `fn_iferror` (bridge.rs:891) — same pattern
- [x] 4.5 Fix `field_reassign_with_default` in `cached_value()` (cell.rs:476) — use `..Default::default()`

## 5. write_cell_xml defensive fix

- [x] 5.1 Change standalone `if` to `else if` for `error_value` check in `write_cell_xml` Formula arm (src/writer/xlsx.rs)

## 6. Verification

- [x] 6.1 `cargo clippy --features formula-eval -- -D warnings` passes clean
- [x] 6.2 `cargo test --features formula-eval` all tests pass (existing + new)
- [x] 6.3 `openspec validate --change formula-eval-correctness-gaps` passes
