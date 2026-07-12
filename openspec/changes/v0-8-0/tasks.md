## 1. Model

- [x] 1.1 Create `src/model/data_validation.rs` with `DataValidation` napi struct (14 fields), `validate()` method, and inline tests
- [x] 1.2 Add `pub mod data_validation` to `src/model/mod.rs`
- [x] 1.3 Add `data_validations: Arc<Mutex<Vec<DataValidation>>>` field to `Worksheet`, initialize in `new()`
- [x] 1.4 Implement `dataValidations` getter, `addDataValidation(dv)`, `getDataValidation(sqref)`, `removeDataValidation(sqref)` napi methods
- [x] 1.5 Add internal `insert_data_validation(dv)` (reader) and `get_data_validations()` (writer) methods

## 2. Writer

- [x] 2.1 Emit `<dataValidations>` block in `write_sheet_xml` after `<hyperlinks>`
- [x] 2.2 Collect data validations from worksheet; thread through to `write_sheet_xml`

## 3. Reader

- [x] 3.1 Implement `parse_sheet_data_validations` — opens zip, reads each sheet XML
- [x] 3.2 Implement `parse_datavalidations_from_xml` — quick_xml parse of `<dataValidation>` elements
- [x] 3.3 Wire parse into `workbook_inner_from_bytes` (attach validations to worksheets by index)

## 4. Types / Tests

- [x] 4.1 Add `DataValidation` interface and 4 Worksheet methods to `index.d.ts`
- [x] 4.2 Write JS tests in `__test__/data-validation.test.ts` (add/get/remove/round-trip/exceljs cross-check)
- [x] 4.3 `cargo test` (240 pass) + `pnpm test` (89 pass) green

## 5. Docs

- [x] 5.1 Update `docs/spec.md` §9.3 (remove Future item, add §9.2.2)
- [x] 5.2 Update `CHANGELOG.md` with `## [0.8.0]` Added section

## 6. Ship

- [ ] 6.1 Open PR, review, merge to main
- [ ] 6.2 Tag `v0.8.0` and push
- [ ] 6.3 Release workflow publishes to npm
- [ ] 6.4 `openspec archive v0-8-0`
