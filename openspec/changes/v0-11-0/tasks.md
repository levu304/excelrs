## 1. Model & FFI scaffolding

- [x] 1.1 Add `auto_filter: Option<String>` to inner `Worksheet` (shared `Arc<Mutex<>>` state, mirroring `data_validations`).
- [x] 1.2 Add `views: Arc<Mutex<Vec<SheetView>>>` and `protection: Arc<Mutex<Option<SheetProtection>>>` to inner `Worksheet` (`SheetView`/`SheetProtection` model structs with `state`/`xSplit`/`ySplit`/`topLeftCell`/`activePane` and boolean flags).
- [x] 1.3 Expose getters/setters on the `napi` `Worksheet`: `auto_filter`/`set_auto_filter`, `views`/`set_views`, `protection`/`set_protection`; keep `src/lib.rs`, `index.d.ts`, `native.d.ts` in sync.
- [x] 1.4 Add `hyperlinks` reader unit-model test helper (cell already carries `hyperlink`/`hyperlink_text`; no model change needed for the write shape).

## 2. Reader (archive-backed parsers)

- [x] 2.1 Add `parse_sheet_auto_filters` reading `<autoFilter ref>` from each `xl/worksheets/sheetN.xml` and returning per-sheet range strings.
- [x] 2.2 Add `parse_sheet_views` reading `<sheetViews><sheetView state><pane xSplit/ySplit/topLeftCell/activePane>` and returning per-sheet view descriptors.
- [x] 2.3 Add `parse_sheet_protection` reading `<sheetProtection>` boolean flags (OOXML bool convention) into per-sheet protection descriptors.
- [x] 2.4 Add `parse_sheet_hyperlinks` + `parse_sheet_rels` to map `<hyperlink ref r:id>` → URL via `xl/worksheets/_rels/sheetN.xml.rels`, producing `(ref, url)` pairs.
- [x] 2.5 Wire `2.1`–`2.4` into `workbook_to_inner_model`: set `ws.auto_filter`, push `views`, set `protection`, and apply hyperlink `CellValue`s at their `ref` cells (reusing `value_type: "Hyperlink"`).

## 3. Writer (schema-ordered emission)

- [x] 3.1 Emit `<autoFilter ref>` at the CT_Worksheet position (after `sheetProtection`, before `mergeCells`/`dataValidations`/`hyperlinks`); omit when unset.
- [x] 3.2 Emit `<sheetViews><sheetView state><pane …/></sheetView></sheetViews>` immediately after `<dimension>`; emit attributes only when present; omit when no views.
- [x] 3.3 Emit `<sheetProtection …/>` after `<sheetViews>` (before `autoFilter`); emit boolean flags as `="1"` only when true; omit when unprotected.
- [x] 3.4 Verify hyperlink write path already round-trips the `{ text, hyperlink }` shape (no change expected); confirm `<hyperlinks>` ordering stays schema-valid.

## 4. Round-trip tests & fixtures

- [x] 4.1 Add reader unit tests for each element against minimal sheet XML snippets (autoFilter / sheetViews / sheetProtection / hyperlinks + rels).
- [x] 4.2 Add an integration round-trip fixture: build a workbook with all four features, write, read back, assert `auto_filter`, `views`, `protection`, and hyperlink `CellValue` survive.
- [x] 4.3 Assert CT_Worksheet element ordering is valid (Excel/LibreOffice-tolerant) in the generated sheet XML (elements placed in correct schema sequence in writer).

## 5. Release bookkeeping

- [x] 5.1 Bump crate version to `0.11.0` in `Cargo.toml` (and `package.json` if the Node package version is coupled).
- [x] 5.2 Update `CHANGELOG.md` with the v0.11.0 entry covering all four features.
- [x] 5.3 Update ROADMAP.md parity matrix: hyperlinks (read), auto-filter, freeze panes, sheet protection → `shipped` (v0.11.0).
- [ ] 5.4 Archive this change via `openspec archive v0-11-0` so the four capability specs and the `exceljs-parity` delta land in `openspec/specs/` (post-merge cleanup).
