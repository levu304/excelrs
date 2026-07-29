## 1. Fix formula persistence (design D1)

- [x] 1.1 In `set_value` (`src/model/cell.rs`), after assigning `inner.value`, set `inner.formula = if inner.value.value_type == "Formula" { inner.value.formula.clone() } else { None }` so formulas set via the object setter reach `CellInner.formula` (the writer's `<f>` source).
- [x] 1.2 Confirm the `Date` (path 1) and primitive/JSON arms also clear stale `inner.formula` via the same assignment (reassignment safety).

## 2. Remove dead parsing (design D2)

- [x] 2.1 Delete the `formula`, `hyperlink`, `hyperlink_text`, and `rich_text` reads in the `valueType` arm (`cell.rs:336/338/341`); keep only `number`/`string`/`boolean`/`error_value`/`date_serial`.

## 3. Validate valueType discriminant (design D3)

- [x] 3.1 In the `valueType` arm, reject `vt` not in the known set (`Number|String|Boolean|Formula|Error|Hyperlink|RichText|Date|Null|Merge`) by returning `Err(ExcelrsError::...)`.
- [x] 3.2 Verify the known-set check matches the `CellValue` constructors and writer `value_type` arms exactly.

## 4. Tests

- [x] 4.1 `__test__/cell.test.ts` or `__test__/rich-text.test.ts`: add a formula-via-object test that writes the workbook, reads it back, and asserts the cell is a formula with the correct `formula` string (covers spec scenario "Formula persists through XLSX write and read-back").
- [x] 4.2 Add a test asserting `cell.value = { valueType: "Banana", number: 5 }` raises an error (invalid valueType), not a silent empty cell.
- [x] 4.3 Add a test: `cell.value = { formula: "SUM(A1:A2)" }` then `cell.value = 42` → cell is Number with no `<f>` emitted.

## 5. Verify

- [x] 5.1 Run `cargo test` and `npm test` (vitest) — all green.
- [x] 5.2 Run `openspec validate fix-cell-setter-dispatch` to confirm artifacts are consistent.
