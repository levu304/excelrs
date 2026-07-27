## 1. SheetView — add showGridLines

- [x] 1.1 Add `show_grid_lines: Option<bool>` field to `SheetView` struct in
      `src/model/sheet_view.rs`
- [x] 1.2 Parse `showGridLines` attribute in `parse_views_from_xml` reader
      (`src/reader/xlsx.rs`, attribute match block at line ~760)
- [x] 1.3 Emit `showGridLines` attribute in `emit_sheet_views` writer
      (`src/writer/xlsx.rs`, `<sheetView` open-tag format at line ~1104)

## 2. AddWorksheetOptions struct

- [x] 2.1 Create `AddWorksheetOptions` napi-object struct in
      `src/model/workbook.rs` with fields: `page_setup`, `views`,
      `header_footer`, `protection`, `auto_filter`
- [x] 2.2 Add `AddWorksheetOptions` interface to `index.d.ts` matching the
      Rust struct shape (camelCase field names)

## 3. Wire options into addWorksheet

- [x] 3.1 Change `Workbook::add_worksheet` signature to accept
      `Option<AddWorksheetOptions>` and pass through to inner
      (`src/model/workbook.rs`)
- [x] 3.2 Apply options via inner setters in the napi wrapper (`src/model/workbook.rs`)
- [x] 3.3 Update `index.d.ts` `addWorksheet(name: string)` to
      `addWorksheet(name: string, options?: AddWorksheetOptions)`

## 4. Tests

- [x] 4.1 Unit test: `WorkbookInner::add_worksheet` with options applies
      page_setup and views to the returned worksheet
- [x] 4.2 Unit test: `add_worksheet` with `None` options produces same result
      as current single-arg call
- [x] 4.3 Unit test: SheetView show_grid_lines round-trips through XML writer
      → XML reader
- [x] 4.4 Integration test (napi): JS caller passes options to addWorksheet and
      reads them back
- [x] 4.5 Run full test suite: `pnpm test` passes
