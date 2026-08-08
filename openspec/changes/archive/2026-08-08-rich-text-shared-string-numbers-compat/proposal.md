## Why

A rich-text cell written via `cell.value = { richText: [...] }` renders with the
**wrong font (Calibri) in Apple Numbers**, even though the per-run font (e.g.
"Times New Roman") is explicitly set. Investigation confirmed the writer output
is correct, not the bug: the generated `xl/worksheets/sheet1.xml` contains
`<rFont val="Times New Roman"/>` on every run (verified by
`unzip -p output.xlsx … | grep rFont`). The XML is schema-valid inline-string
rich text (`t="inlineStr"` with per-run `<rPr><rFont/>`). Apple Numbers' XLSX
importer ignores inline-string rich-text run fonts and falls back to the cell's
default font, which is Calibri. This is a cross-app compatibility gap, not a
writer correctness defect.

Note: the prior commit `c5334e15` ("rich-text run without rFont no longer reads
back Calibri") was a **reader-only** fix — it stopped the *parser* from leaking
Calibri on read-back. It could not and did not touch this write/Numbers path,
which is why it "did not fix" the reported issue.

## What Changes

- The rich-text writer SHALL emit rich text through the **shared-string table**
  (`xl/sharedStrings.xml`) as `<si><r><rPr>…</rPr><t>…</t></r></si>`, and the
  cell SHALL reference it with `t="s"` + `<v>idx</v>` — instead of the current
  inline `t="inlineStr"` form.
- This aligns the write path with Excel's own output and with the existing
  `rich-text` read path (v0.12.0 already parses shared-string `<si><r>` runs).
- The JavaScript API is unchanged: `cell.value = { richText: [...] }` keeps the
  same shape.
- Round-trip (`write → read back`) must still preserve run text and per-run
  fonts.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities

- `rich-text`: adds the requirement that the writer emits rich text via shared
  strings (for cross-app / Numbers compatibility), changing the write format
  from inline strings to shared strings.

## Impact

- `src/writer/xlsx.rs` — the `CellType::RichText` arm (currently emits inline
  `<is>…</is>`) is rerouted to the shared-string table.
- Shared-string table construction (currently keyed by plain `String`) must gain
  a rich-text key variant so identical rich-text runs dedupe and serialize as
  `<si><r>…</r></si>`.
- No public API, ABI, or dependency changes. Shared-string output is smaller and
  more broadly compatible (Excel, LibreOffice, Numbers) than inline strings.
