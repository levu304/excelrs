# Tasks: Style Setter Type Safety

TDD cycle: Red (write failing tests) → Green (make tests pass) → Refactor (verify types).

## 1. Red: Write tests for `Option<Style>` contract

- [x] 1.1 Add unit tests to `Cell` test module in `src/model/cell.rs`:
  - `test_set_style_some`: `cell.set_style(Some(valid_style))` stores style
  - `test_set_style_none`: `cell.set_style(None)` resets to Normal
  - `test_set_style_empty`: `cell.set_style(Some(empty_style))` resets to Normal (empty-object path)
- [x] 1.2 Add unit tests to `Row` test module in `src/model/row.rs`:
  - `test_row_set_style_some` / `test_row_set_style_none`
- [x] 1.3 Add unit tests to `Column` test module in `src/model/column.rs`:
  - `test_column_set_style_some` / `test_column_set_style_none`
- [x] 1.4 Verify these tests **fail to compile** against current `serde_json::Value` signatures — confirmed 7 compilation errors

## 2. Green: Change setter signatures

- [x] 2.1 Change `Cell::set_style` parameter from `serde_json::Value` to `Option<Style>`, replace body with match pattern
- [x] 2.2 Change `Row::set_style` same pattern as Cell
- [x] 2.3 Change `Column::set_style` same pattern as Cell
- [x] 2.4 Change `Worksheet::set_cell_style` same pattern — delegates to `Cell::set_style`
- [x] 2.5 Run `cargo test` — **375 tests pass**, confirming Green phase

## 3. Refactor: Convert old test callers

- [x] 3.1 Convert `test_cell_style_mutation_persists_through_clone` (worksheet.rs:1364): replaced with `Style` struct construction
- [x] 3.2 Convert `test_duplicate_row_include_style_false_does_not_corrupt_source` (worksheet.rs:1477): replaced with `Style` struct construction
- [x] 3.3 Run `cargo test` — all tests pass, no `serde_json::json!` passed to `set_style`

### Bonus (discovered during compilation)

- [x] 3.4 Convert 8 additional `set_style(json!(...))` callers in `src/writer/xlsx.rs` and `src/writer/styles.rs` test modules to use `Style` struct construction

## 4. Verify generated TypeScript declarations

- [x] 4.1 Run build command to regenerate `index.d.ts`
- [x] 4.2 Confirm `set style(val: any)` replaced by `set style(val: Style | undefined | null)` on Cell, Row, Column
- [x] 4.3 Confirm `setCellStyle(row: number, col: number, style?: Style | undefined | null): void` on Worksheet
- [x] 4.4 Confirm no `any` remains on any style setter in the generated declarations
