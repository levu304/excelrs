## 1. CSV module (parse + serialize)

- [x] 1.1 Create `src/csv.rs` with `parse_csv(buf, delimiter) -> Workbook` and `serialize_csv(wb, delimiter, withBom) -> Vec<u8>` (manual RFC 4180, no new dep)
- [x] 1.2 Unit tests for the parser (quoted fields, embedded comma/newline, custom delimiter) and serializer (quoting, BOM)

## 2. Handle + model wiring

- [x] 2.1 Add `WorkbookCsv` handle (mirror `WorkbookXlsx`): async `read`/`readFile`/`write`/`writeFile`
- [x] 2.2 Add `csv` getter on `Workbook` returning `WorkbookCsv` sharing the inner `Arc<Mutex<>>`
- [x] 2.3 Register the module in `src/lib.rs` / `src/xlsx/mod.rs`

## 3. Types / tests

- [x] 3.1 Add `WorkbookCsv` interface + `csv` getter to `index.d.ts`
- [x] 3.2 JS vitest: csv round-trip (write→read preserves values), numeric inference, exceljs cross-check

## 4. Docs

- [x] 4.1 Update `docs/spec.md` §9.3 (remove CSV from Future, add §9.2.3)
- [x] 4.2 Update `CHANGELOG.md` with `## [0.9.0]` Added section
- [x] 4.3 Update `README.md` limitations (CSV single-sheet, no type preservation)

## 5. Ship

- [ ] 5.1 Open PR, review, merge to main
- [ ] 5.2 Bump version to 0.9.0 in `Cargo.toml` / `package.json`
- [ ] 5.3 Tag `v0.9.0` and push
- [ ] 5.4 Release workflow publishes to npm
- [ ] 5.5 `openspec archive v0-9-0-csv-read-write`
