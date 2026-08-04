# Rich-text shared-strings read (TDD task breakdown)

TDD contract: every feature lists its **tests first**, then implementation.
Tests are named and asserted concretely. Implement only to make the listed
tests pass, smallest-first.

## Test budget (target)

- Rust: ~10 new (`shared_strings_parse` ~4, `overlay` ~3, round-trip ~3).
- JS: ~2 new in `__test__/rich-text.test.ts` (ExcelJS-produced shared-strings rich text read-back).
- Baseline before start: current Rust + JS baseline (check `cargo test` count).

---

## A. Parse rich-text entries from `xl/sharedStrings.xml` (new function)

### A-tests

- [x] `A1 test_parse_shared_string_rich_text_extracts_runs` — inline `sharedStrings.xml` with `<si><r><rPr><rFont val="Arial"/><sz val="12"/></rPr><t>Hi</t></r></si>` at index 0 → `HashMap[0] = vec![RichTextRun { text: "Hi", font: Some(Font { name: "Arial", size: 12 }) }]`.
- [x] `A2 test_parse_shared_string_skips_plain_si` — `<si><t>plain</t></si>` → absent from HashMap.
- [x] `A3 test_parse_shared_string_multiple_runs` — `<si><r><rPr><b/></rPr><t>A</t></r><r><t>B</t></r></si>` → runs = [{text:"A",font:b}, {text:"B",font:none}].
- [x] `A4 test_parse_shared_string_no_file` — zip without `sharedStrings.xml` → empty HashMap (not error).

### A-impl

- [x] `A.1` Add `parse_shared_string_rich_text` (implemented with `apply_rpr_child` helper)
- [x] `A.2` Extract `apply_rpr_child` rPr→Font helper parsing from `parse_inline_str_rich_text_with` into `parse_rpr_font` so both inline and shared-strings paths share it.

---

## B. Overlay shared-string rich text onto cells (reader step)

### B-tests

- [x] `B1 test_overlay_shared_string_rich_text_replaces_string` — workbook bytes with a `<c t="s"><v>0</v></c>` cell + sharedStrings rich text at index 0 → after `workbook_inner_from_bytes`, cell ValueType is RichText, runs + font match.
- [x] `B2 test_overlay_shared_string_plain_string_unchanged` — `<c t="s"><v>1</v></c>` referencing a plain `<si><t>text</t></si>` → cell stays String (not RichText).
- [x] `B3 test_overlay_inline_str_not_affected` — an inline-string rich text cell (`<c t="inlineStr">`) and a shared-string rich text cell coexist → both get RichText, fonts correct.

### B-impl

- [x] `B.1` Wire Step 3.10b into workbook_inner_from_bytes in `workbook_inner_from_bytes`: call `parse_shared_string_rich_text(data)` → `rich_strings` map.
- [x] `B.2` Implement `overlay_shared_string_rich_text`
- [x] `B.3` Placed after Step 3.10, before Step 3.11

---

## C. JS round-trip with Excel-generated shared-strings rich text

### C-tests

- [x] `C1 test read rich text from spreadsheet-shared-strings file` — generate a workbook via `exceljs` (devDependency) with rich text runs, write to buffer, read via `new Workbook().xlsx.read(buf)`, assert `cell.type === 'RichText'`, `richText[0].font.name` matches.
- [x] `C2 test exceljs write + exceljs read preserves rich text` — round-trip through exceljs itself, then read back via excelrs, verify fonts survived.

### C-impl

- [x] `C.1` No code changes needed — parser handles ExcelJS output (D1+D2 should make this pass) — implement only if test fails.
- [x] `C.2` N/A — no calamine issues or missing file path, adjust `parse_shared_string_rich_text` to handle the actual sharedStrings entry count/structure.

---

## D. Verify existing rich-text tests still pass

- [x] `D1 rich-text.test.ts: 13 tests pass (11 existing + 2 new) — no regression`
- [x] `D2 parse_inline_str_rich_text tests: pass — no regression`
- [x] `D3 test_rich_text_roundtrip (Rust): pass — no regression`
