# Tasks: v1.1.0 — Tables

## 1. Table model

- [x] 1.1 Add `Table` / `TableColumn` / `TableRow` / `TableStyle` model types in `src/model/table.rs` (name, display_name, ref, header_row, totals_row, columns, rows, style, autofilter_ref)
- [x] 1.2 Store tables on `Worksheet` (e.g. `Vec<Table>`) with name-uniqueness validation

## 2. Writer — table part + relationship

- [x] 2.1 Writer: emit `xl/tables/tableN.xml` (`<table>` with `<autoFilter>`, `<tableColumns>`, `<tableStyleInfo>`, `totalsRowShown`) at the schema-correct position; omit parts when no tables
- [x] 2.2 Writer: register a `table` relationship in `xl/worksheets/_rels/sheetN.xml.rels` for each table (reuse existing rels manager)

## 3. Reader — table part + relationship

- [x] 3.1 Reader: resolve sheet `.rels` → `xl/tables/tableN.xml`, parse into the `Table` model (name, displayName, ref, headerRow, totalsRow, columns w/ totalsRowLabel/Function, rows, style, autofilter_ref)
- [x] 3.2 Reader: populate `ws` table list; leave empty when sheet has no table part

## 4. napi bridge — add/get/remove + cell population

- [x] 4.1 Add `Worksheet.addTable(opts)` writing header + data (+ optional totals) values into the referenced cells, then registering the table model (mirror `add_image` at `src/model/worksheet.rs:493`)
- [x] 4.2 Add `Worksheet.getTable(name)` / `getTables()` / `removeTable(name)` (removeTable deletes model + part + rel, leaves cells intact)
- [x] 4.3 Expose `Table` / `TableColumn` / `TableRow` / `TableStyle` JS types and `addTable`/`getTable(s)`/`removeTable` in `src/lib.rs` + `index.d.ts`

## 5. Round-trip fixtures

- [x] 5.1 Add ExcelJS-authored round-trip fixture (`ws.addTable` → write → read; name/ref/columns/rows/style/autofilter_ref match)
- [x] 5.2 Add Excel-authored `.xlsx` round-trip fixture (read → write → read preserves table)
- [x] 5.3 Validate `removeTable` leaves underlying cells intact

## 6. Release & parity bookkeeping

- [x] 6.1 Bump `package.json` `1.0.0` → `1.1.0`; update `CHANGELOG.md` and `ROADMAP.md` parity matrix (`tables` → `shipped`)
- [x] 6.2 Sync the `exceljs-parity` spec delta and archive the change when all fixtures pass
