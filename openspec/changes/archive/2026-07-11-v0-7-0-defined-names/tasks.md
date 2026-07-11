# v0.7.0 — Defined Names (TDD task breakdown)

TDD contract: every feature lists its **tests first**, then implementation.
Tests are named and asserted concretely. Implement only to make the listed
tests pass, smallest-first.

## Test budget (target)

- Rust: ~15 new (`defined_name` ~4, reader ~4, writer ~3, round-trip ~4).
- JS: ~6 new in `__test__/defined-names.test.ts`.
- Baseline before start: 173 Rust + 69 JS (CHANGELOG 0.6.0). Target ≈ 188 Rust + 75 JS.

---

## A. `DefinedName` model (`src/model/defined_name.rs`, new file)

### A-tests (write BEFORE impl)

- [ ] `A1 test_defined_name_new` — `DefinedName { name: "X", value: "1", sheet: None }` fields match.
- [ ] `A2 test_defined_name_with_sheet` — `DefinedName { name: "Y", value: "Sheet1!$A$1", sheet: Some("Sheet1".into()) }` fields match.
- [ ] `A3 test_workbook_inner_defined_names_default_empty` — `WorkbookInner::new().defined_names.is_empty()`.
- [ ] `A4 test_workbook_inner_defined_names_roundtrip_fields` — create `WorkbookInner`, set `defined_names = vec![...]`, assert `(...)` — validates the field exists and is readable.

### A-impl

- [ ] `A.1` Create `src/model/defined_name.rs` with `#[napi(object)] pub struct DefinedName { pub name: String, pub value: String, pub sheet: Option<String> }`.
- [ ] `A.2` Add `pub mod defined_name;` to `src/model/mod.rs`.
- [ ] `A.3` Add `pub defined_names: Vec<DefinedName>` to `WorkbookInner` struct (default `Vec::new()`).

## B. Reader: parse defined names from `xl/workbook.xml` (`src/reader/workbook.rs`, new file)

### B-tests

- [ ] `B1 test_parse_defined_names_global` — inline workbook.xml with one global `<definedName>` → returns vec with one entry, name+value match, sheet=None.
- [ ] `B2 test_parse_defined_names_sheet_scoped` — inline XML with `localSheetId="0"` and sheet_names=["Sheet1"] → name, value match, sheet=Some("Sheet1").
- [ ] `B3 test_parse_defined_names_local_id_out_of_range` — `localSheetId="99"` with 1 sheet → sheet=None (clamped).
- [ ] `B4 test_parse_defined_names_empty` — workbook.xml without `<definedNames>` → empty vec.

### B-impl

- [ ] `B.1` Create `src/reader/workbook.rs` with `parse_defined_names(data: &[u8], sheet_names: &[String]) -> Result<Vec<DefinedName>, ExcelrsError>`. Opens zip via `Cursor`, reads `xl/workbook.xml`, parses `<definedName>` elements with quick_xml. `<definedNames>` absent → empty Vec.
- [ ] `B.2` Add `pub mod workbook;` to `src/reader/mod.rs`.
- [ ] `B.3` Wire into `workbook_inner_from_bytes` in `src/reader/xlsx.rs`: after styles parse, call `reader::workbook::parse_defined_names(data, &sheet_names)` and set `inner.defined_names`.

## C. Writer: emit defined names in `xl/workbook.xml` (`src/writer/xlsx.rs`)

### C-tests

- [ ] `C1 test_write_defined_name_global` — workbook with one global defined name → re-read via `workbook_inner_from_bytes` → same name+value+sheet=None.
- [ ] `C2 test_write_defined_name_sheet_scoped` — workbook with sheet-scoped name → re-read: sheet=Some("Sheet1").
- [ ] `C3 test_write_no_defined_names_omits_element` — empty `defined_names` → raw sheet XML checked for absence of `<definedNames>`.
- [ ] `C4 test_write_defined_name_multiple` — workbook with 2 names → re-read yields both in order.

### C-impl

- [ ] `C.1` Modify `write_workbook_xml` in `src/writer/xlsx.rs` to accept `&[DefinedName]` and `&[Worksheet]` (for sheet name→index mapping). Emit `<definedNames>` after `</sheets>` when non-empty.
- [ ] `C.2` Sheet-scoped: lookup `worksheet.name()` to find 0-based index; emit `localSheetId="N"`. Not found → omit attribute.
- [ ] `C.3` Pass `defined_names` from `WorkbookInner` in `workbook_to_bytes`.
- [ ] `C.4` Text content escaped via `quick_xml::escape::escape`.

## D. NAPI: `Workbook` methods + `index.d.ts`

### D-tests

Write tests in `src/model/workbook.rs` (unit) and `src/model/workbook_inner.rs`.

- [ ] `D1 test_napi_add_defined_name_global` — `Workbook::add_defined_name("Rate", "0.08", None)` → defined_names len==1, name/value match.
- [ ] `D2 test_napi_add_defined_name_sheet` — with `sheet=Some("Sheet1")` → entry has sheet field.
- [ ] `D3 test_napi_add_defined_name_upsert` — add same name twice → len==1, value updated.
- [ ] `D4 test_napi_remove_defined_name` — add then remove → len==0.
- [ ] `D5 test_napi_remove_absent_noop` — remove non-existent → len unchanged.
- [ ] `D6 test_napi_get_defined_name` — add then get by name → returns the entry.
- [ ] `D7 test_napi_get_defined_name_missing` → returns `None`.

### D-impl

- [ ] `D.1` Add to `Workbook` (napi impl block in `src/model/workbook.rs`):
  - getter `defined_names` returning `Vec<DefinedName>` (clone snapshot).
  - `add_defined_name(name: String, value: String, sheet: Option<String>)` — upserts.
  - `remove_defined_name(name: String, sheet: Option<String>)` — removes matching.
  - `get_defined_name(name: String, sheet: Option<String>) -> Option<DefinedName>`.
- [ ] `D.2` Add `DefinedName` type to `index.d.ts`:

  ```typescript
  export interface DefinedName {
    name: string
    value: string
    sheet?: string | null
  }
  ```

- [ ] `D.3` Add methods to `Workbook` declaration in `index.d.ts`: `definedNames`, `addDefinedName`, `removeDefinedName`, `getDefinedName`.

## E. JS integration tests (`__test__/defined-names.test.ts`, new file)

### E-tests

- [ ] `E1` Add global defined name → write buffer → read back via excelrs → name/value match.
- [ ] `E2` Add sheet-scoped defined name → write → read back → sheet name intact.
- [ ] `E3` Remove defined name → write → read back → absent.
- [ ] `E4` Two names round-trip (order preserved).
- [ ] `E5` exceljs file with defined names read by excelrs: create workbook in exceljs with `workbook.definedNames.add(...)`, write buffer, read with excelrs → names match.
- [ ] `E6` Round-trip via excelrs: add → write → exceljs read → names match.

## F. Edge cases & hardening

- [ ] `F1` Defined name value with special XML chars (`&<>"`) → escaped on write, unescaped on read.
- [ ] `F2` Defined name with empty value (`<definedName name="X"></definedName>`) → value="" preserved.
- [ ] `F3` Very long value (1024+ chars) → read/write round-trips correctly.
- [ ] `F4` `cargo test` green (≥188 tests).
- [ ] `F5` `pnpm test` green (≥75 tests).

## G. Docs / spec

- [ ] `G1` `docs/spec.md` §1: version → v0.7.0, brief scope note.
- [ ] `G2` `docs/spec.md`: new §6.X "Defined Names" subsection covering read, write, and API.
- [ ] `G3` `docs/spec.md` §9.3: mark "Defined names (named ranges)" as shipped in v0.7.0.
- [ ] `G4` `CHANGELOG.md`: add v0.7.0 entry (Added: defined names read+write+API).

## Verification gate (all before merge)

- [ ] `cargo test` green; `cargo clippy -- -D warnings`; `cargo fmt -- --check`.
- [ ] `pnpm test` green (incl. E1–E6).
- [ ] `cargo test` count ≥188; `pnpm test` count ≥75.
