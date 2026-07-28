## 1. Add ColumnInput struct

- [x] 1.1 Add `ColumnInput` struct to `src/model/column.rs` with `#[napi(object)]`, all `Option` fields, and `Default` derive
- [x] 1.2 Verify napi-rs generates `ColumnInput` interface in `native.d.ts` on next build

## 2. Update set_columns signature

- [x] 2.1 Change `set_columns` in `src/model/worksheet.rs` from `serde_json::Value` to `Vec<ColumnInput>`
- [x] 2.2 Replace `serde_json::from_value(cols)` with `cols.into_iter().map(...)` conversion to `Vec<Column>`
- [x] 2.3 Preserve col_num auto-assignment, duplicate detection, and style validation logic unchanged

## 3. Update test callers

- [x] 3.1 Update `src/writer/xlsx.rs` callers of `set_columns` that pass `serde_json::Value` to pass `Vec<ColumnInput>` instead
- [x] 3.2 No test callers of `set_columns` in `src/model/worksheet.rs` — only in `xlsx.rs` (covered by 3.1)
- [x] 3.3 `serde_json` still used in `worksheet.rs` for `add_row`, `insert_row`, `splice_rows`, etc. — keeping import; removed from `set_columns` path only

## 4. Rebuild and verify types

- [x] 4.1 Run `pnpm build` — compilation succeeds
- [x] 4.2 Inspect `native.d.ts` — `setColumns(cols: Array<ColumnInput>)` and `ColumnInput` interface present
- [x] 4.3 Inspect `index.d.ts` — same type signature now present
- [x] 4.4 Run `pnpm test` — 136/136 tests pass
- [x] 4.5 Run `cargo clippy` — no warnings

## 5. Cleanup

- [x] 5.1 `serde_json` still used elsewhere in `worksheet.rs` (not removable)
- [x] 5.2 No remaining `serde_json::Value` references in `set_columns` code path
- [x] 5.3 Run full test suite one final time — 136/136 pass
