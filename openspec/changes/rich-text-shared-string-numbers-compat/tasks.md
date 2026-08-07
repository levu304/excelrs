## 1. Baseline & Validation Gate

- [ ] 1.1 Build a fresh native addon (`napi build --platform --release` / `pnpm rebuild`), run the KIS rich-text repro, open `output.xlsx` in Apple Numbers — confirm the rich-text cell renders Calibri (documents current broken state and rules out a stale addon as the cause)
- [ ] 1.2 Capture the current inline XML as before-state reference: `unzip -p output.xlsx xl/worksheets/sheet1.xml | grep -o 'rFont val="[^"]*"'`

## 2. Shared-string key/table extension

- [ ] 2.1 Define a `SharedString` enum (or equivalent) with `Plain(String)` and `Rich(Vec<RichTextRun>)` variants; update `build_shared_strings` (src/writer/xlsx.rs:374) so `string_table`/`string_indices` carry the new key type instead of plain `String`
- [ ] 2.2 Insert rich-text cells into the shared-string table keyed by their run content, preserving dedupe via `or_insert_with` so identical runs share one entry

## 3. Serialize rich text in sharedStrings.xml

- [ ] 3.1 Update `write_shared_strings` (src/writer/xlsx.rs:1071) to emit `<si><r><rPr>…</rPr><t>…</t></r></si>` for `Rich` entries, reusing the existing per-run `<rPr>` serialization already used inline
- [ ] 3.2 Emit `xml:space="preserve"` on a run's `<t>` element when its text has leading/trailing whitespace or a newline (so the user's `"B: (11) = (7) + (10)\n"` keeps its trailing newline)

## 4. Reroute the writer

- [ ] 4.1 Change `write_cell_xml`'s `CellType::RichText` arm (src/writer/xlsx.rs:1878) to look up the shared-string index and emit `t="s"` + `<v>idx</v>` instead of inline `<is>…</is>`

## 5. Tests

- [ ] 5.1 Unit test: a rich-text cell is written as `t="s"` and `xl/sharedStrings.xml` contains `<si><r><rPr><rFont val="…"/></rPr><t>…</t></r></si>`
- [ ] 5.2 Unit test: write → read back preserves run text and per-run `font`
- [ ] 5.3 Unit test: identical rich text across two cells deduplicates to one shared-string index
- [ ] 5.4 Unit test: a run with trailing-newline text is emitted with `xml:space="preserve"`

## 6. Acceptance (Numbers)

- [ ] 6.1 Rebuild the addon, run the repro, open `output.xlsx` in Numbers — confirm rich-text runs render in their specified fonts (not Calibri). If still Calibri, suspend the change and report per design Risks (shared strings may not be the cure)
