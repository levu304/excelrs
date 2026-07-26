## 1. Add missing edge-case tests (Red phase)

- [x] 1.1 Add `test_row_set_style_empty_object` to Row test module: `Style::default()` → cleared via `is_empty()` guard
- [x] 1.2 Add `test_column_set_style_empty_object` to Column test module: `Style::default()` → cleared via `is_empty()` guard
- [x] 1.3 Add `test_cell_set_style_rejects_invalid` to Cell test module: invalid style (e.g., empty `num_fmt` string) → `is_err()`
- [x] 1.4 Add `test_row_set_style_rejects_invalid` to Row test module: same pattern
- [x] 1.5 Add `test_column_set_style_rejects_invalid` to Column test module: same pattern
- [x] 1.6 Run `cargo test --lib` — all tests pass (these tests target existing `Option<Style>` API, so they pass from the start)

## 2. Extract `apply_style` helper

- [x] 2.1 Add `pub(crate) fn apply_style(dest: &mut Option<Style>, val: Option<Style>) -> napi::Result<()>` to `src/model/style.rs` with the combined match pattern (`None | Some(ref s) if s.is_empty()`) and bare `s.validate()?` (no `map_err`)
- [x] 2.2 Update `Cell::set_style` body to `apply_style(&mut inner.style, val)`
- [x] 2.3 Update `Row::set_style` body to `apply_style(&mut *guard, val)`
- [x] 2.4 Update `Column::set_style` body to `apply_style(&mut self.style, val)`
- [x] 2.5 Update `Worksheet::set_columns` style-validation loop (lines 503-511) to use `apply_style(&mut col.style, style)?`
- [x] 2.6 Run `cargo test --lib` — 375+ tests pass, confirming refactor preserved behavior

## 3. Fix `add_data_validation` `map_err`

- [x] 3.1 Replace `dv.validate().map_err(|e| napi::Error::from_reason(e.to_string()))?` with `dv.validate()?` in `src/model/worksheet.rs:592`
- [x] 3.2 Run `cargo test --lib` — all pass

## 4. Verify and finalize

- [x] 4.1 Run `cargo test --lib` — all tests pass
- [x] 4.2 Run `cargo clippy -- -D warnings` — no new warnings
- [x] 4.3 Confirm no `map_err(|e| napi::Error::from_reason(e.to_string()))` remains in `src/model/` for `ExcelrsError` types (5 occurrences should be eliminated)
