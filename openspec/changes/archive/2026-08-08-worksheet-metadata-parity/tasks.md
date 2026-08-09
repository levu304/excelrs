## 1. Model fields & types

- [x] 1.1 Add `SheetState` enum (`visible` | `hidden` | `veryHidden`) in `src/model/worksheet.rs` (string-backed, maps to OOXML `state` attr)
- [x] 1.2 Add fields to `Worksheet`: `state`, `tab_color: Option<Color>`, `default_row_height: Option<f64>`, `default_col_width: Option<f64>`, `outline_level_row: Option<u8>`, `outline_level_col: Option<u8>`
- [x] 1.3 Add `#[napi(getter)]`/`#[napi(setter)]` pairs for `state` and `tab_color` (mirror existing `views` pattern)
- [x] 1.4 Add `WorksheetProperties` napi struct + `properties` getter + `setProperties(Partial<WorksheetProperties>)` method on `Worksheet`
- [x] 1.5 Extend `AddWorksheetOptions` (`src/model/workbook.rs`) with `state` and `properties`; apply them in `add_worksheet`

## 2. Reader parsers

- [x] 2.1 Add `parse_sheet_states(data)` — walk `xl/workbook.xml` `<sheet>` elements, capture `state` attr, attach by index
- [x] 2.2 Add `parse_sheet_tab_colors` logic inside `parse_sheet_format_pr` — parse `<sheetPr><tabColor>` per sheet via existing `Color` parser
- [x] 2.3 Add `parse_sheet_format_pr(data, sheet_count)` — parse `<sheetFormatPr>` attrs (defaultRowHeight, defaultColWidth, outlineLevelRow/Col)
- [x] 2.4 Hook all three into `workbook_inner_from_bytes` as new Step 3.x calls (attach by index, matching neighbors)

## 3. Writer emit

- [x] 3.1 Add `state` attribute to `<sheet>` template in `write_workbook_xml`; omit when `visible`
- [x] 3.2 Emit `<sheetPr><tabColor/></sheetPr>` before `<dimension>` in `write_sheet_xml` when a tab color is set
- [x] 3.3 Emit `<sheetFormatPr .../>` between `<sheetViews>` and `<cols>` when any default dimension is set
- [x] 3.4 Reuse existing `Color` serialization for tab color to keep theme/indexed/ARGB correct

## 4. TypeScript surface

- [x] 4.1 Add `WorksheetState` and `WorksheetProperties` interfaces to `index.d.ts` + `dts-header.d.ts` + `native.d.ts`
- [x] 4.2 Add `state` + `properties` fields to `AddWorksheetOptions` and `state`/`tabColor`/`properties` to `Worksheet` in the d.ts

## 5. Tests & conformance

- [x] 5.1 Round-trip fixture: hidden sheet stays hidden (`state="hidden"` in written workbook.xml)
- [x] 5.2 Round-trip fixture: `veryHidden` preserved
- [x] 5.3 Round-trip fixture: tab color survives (`<sheetPr><tabColor>`)
- [x] 5.4 Round-trip fixture: `defaultRowHeight` / `defaultColWidth` / outline levels survive
- [x] 5.5 `addWorksheet(name, { state, properties })` create-path test
- [x] 5.6 LibreOffice / XSD conformance check asserting CT_Worksheet child ordering (sheetPr → dimension → sheetViews → sheetFormatPr → cols → sheetData)

## 6. Validation

- [x] 6.1 `openspec validate worksheet-metadata-parity` passes
- [x] 6.2 `cargo test` (434 passed) and `pnpm test` green for the new fixtures (25/25 worksheet tests pass; 4 pre-existing `cached-formula` failures require the opt-in `formula-eval` feature and are unrelated to this change)
