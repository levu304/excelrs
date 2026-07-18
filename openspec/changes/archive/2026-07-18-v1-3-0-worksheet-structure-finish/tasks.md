# Tasks: v1.3.0 — Worksheet-structure parity finish

## 1. Row/Column outline level model + napi

- [x] 1.1 Add `outlineLevel` (`Arc<Mutex<u8>>`, default 0, clamped 0–7) to `Row` in `src/model/row.rs` with getter/setter; add `Row::renumber(new_number)` updating `number` and each cell's row + address
- [x] 1.2 Add `outlineLevel: u8` serde field (camelCase, default 0) + getter/setter to `Column` in `src/model/column.rs`
- [x] 1.3 Add private `Cell::renumber(new_row)` in `src/model/cell.rs` updating `row` + recomputed `address`
- [ ] 1.4 Expose `Row.outlineLevel`, `Column.outlineLevel` in `src/lib.rs` + `index.d.ts`

## 2. Page breaks model + napi

- [x] 2.1 Add `row_breaks` / `col_breaks` (`Arc<Mutex<BTreeSet<u32>>>`) to `Worksheet` in `src/model/worksheet.rs`; add internal `insert_row_break` / `insert_col_break`
- [x] 2.2 Expose `rowBreaks()` / `setRowBreaks` and `colBreaks()` / `setColBreaks` on `Worksheet` in `src/model/worksheet.rs`, `src/lib.rs`, and `index.d.ts`

## 3. Row insertion / mutation (insertRow / spliceRows / duplicateRow)

- [x] 3.1 Add `Worksheet.insertRow(rowNumber, values?)` — shift rows ≥ `rowNumber` down by 1 via the ordered-`Vec<Row>` renumber (D1), insert a new populated row
- [x] 3.2 Add `Worksheet.spliceRows(start, count, rows?)` — remove `count` rows at `start`, insert provided row-value arrays, renumber
- [x] 3.3 Add `Worksheet.duplicateRow(rowNumber, count, includeStyle)` — insert `count` copies below, copying values and (when `includeStyle`) row/cell styles
- [x] 3.4 Expose the three methods on `Worksheet` in `src/lib.rs` + `index.d.ts`

## 4. Reader — outline levels + breaks

- [x] 4.1 Add `parse_sheet_row_outline_levels` in `src/reader/xlsx.rs` (extend the `<row>` scan used by `parse_row_styles_from_xml`) capturing `outlineLevel` → `insert_row_outline_level`; wire as Step 3.17
- [x] 4.2 Add `parse_sheet_col_outline_levels` scanning `<cols><col min max outlineLevel/></cols>` → `insert_column_outline_level`; wire as Step 3.18
- [x] 4.3 Add `parse_sheet_row_breaks` / `parse_sheet_col_breaks` scanning `<rowBreaks>` / `<colBreaks>` `<brk id="…"/>` → `insert_row_break` / `insert_col_break`; wire as Steps 3.19 / 3.20

## 5. Writer — outline levels + breaks

- [x] 5.1 Emit `outlineLevel="N"` on `<row>` in `write_cells_with_styles` (`src/writer/xlsx.rs`) only when `outlineLevel > 0`
- [x] 5.2 Emit a minimal `<cols>` block (one `<col min max outlineLevel/>` per grouped column) only when ≥1 column has `outlineLevel > 0`
- [x] 5.3 Emit `<rowBreaks>` / `<colBreaks>` (with `<brk id max="16383"|"1048575" man="0"/>`) in schema-correct position (after `hyperlinks`, before `pageMargins`); omit both when empty

## 6. Round-trip fixtures

- [ ] 6.1 ExcelJS-authored fixture: set row/column `outlineLevel`, `rowBreaks`/`colBreaks`, and `insertRow`/`spliceRows`/`duplicateRow` → write → read; assert levels, breaks, and shifted row contents/styles match
- [ ] 6.2 Excel-authored `.xlsx` fixture with grouped rows/columns + manual page breaks → read → write → read; assert grouping + breaks preserved
- [ ] 6.3 Regression fixture: a worksheet with no grouping/breaks produces byte-identical `<row>`/no `<cols>`/no break elements vs. prior output

## 7. Release & parity bookkeeping

- [ ] 7.1 Bump `package.json` `1.2.2` → `1.3.0`; update `CHANGELOG.md` and `ROADMAP.md` parity matrix (remaining v1.x rows → `shipped`)
- [ ] 7.2 Sync the `exceljs-parity` spec delta and archive the change when all fixtures pass
