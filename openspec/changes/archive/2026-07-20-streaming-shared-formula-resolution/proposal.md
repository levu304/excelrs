## Why

v2.1.0 shipped the constant-memory streaming XLSX reader (#26 / PR #27) whose
`src/stream.rs` `parse_sheet_rows` SAX-parses `<sheetData>` row-by-row and
captures inline `<f>` formula text into `StreamValue::Formula`. The
whole-workbook reader (calamine-based, `src/reader/xlsx.rs`) already resolves
shared formulas, because calamine 0.35.0 expands `<f t="shared">` *member* cells
to their translated formula text on read. The streaming reader does not: a
shared-formula member cell (`<c r="B5"><f t="shared" si="0"/></c>`) carries no
inline formula text — the formula string lives only on the master cell — so
`parse_sheet_rows` currently falls through to the member's cached `<v>` value
and emits a number, not a `Formula`. The `streaming-xlsx` spec already promises
"the same fidelity as the whole-workbook reader on the read path," so this is a
fidelity gap, not a new feature. Deferred from the v2.1.0 hardening change
(#25.3) as sub-item #25.2; tracked as GitHub issue #32.

## Changes

- **Shared-formula table (per sheet).** `parse_sheet_rows` maintains a
  `HashMap<u32 /*si*/, (String /*master formula text*/, (u32,u32) /*master cell
  position*/)>`, collected as master cells stream by. `si` is per-sheet scoped
  (the function is called once per worksheet), so the table is naturally bounded
  to the sheet and never leaks across sheets.
- **Master detection.** A `<f t="shared" si="N" ref="...">FORMULA</f>` is the
  master: store `(FORMULA, master_pos)` under `si`. Its own emitted value is the
  formula (offset 0 = identity, so no translation is needed for the master
  itself).
- **Member resolution.** A `<f t="shared" si="N"/>` with no inline text is a
  member: look up `si`; if found, compute
  `offset = (member_pos.row - master_pos.row, member_pos.col - master_pos.col)`
  and emit `StreamValue::Formula(translate(master_text, offset))`. Members
  without a seen master (malformed / forward reference) emit no formula, matching
  calamine's behavior.
- **Reference translation ported from calamine.** Port `replace_cell_names` +
  `Reference` (`Cell` / `Row` / `Column`, with `parse` / `offset` / `format`) +
  `offset_range` + `column_number_to_name` into `src/stream.rs`, adapted to
  `ExcelrsError` and the workbook bounds constants. Relative references shift by
  `offset`; absolute `$A$1` / mixed `A$1` references are preserved; function-name
  tokens and quoted strings are skipped; anything that fails to parse (e.g.
  sheet-qualified `Sheet1!A1`) is copied verbatim.
- **On-the-fly offset (memory improvement over calamine).** calamine materializes
  a full `offset_map` over the entire `ref` range (O(range) entries); we compute
  the single member offset directly, so memory is bounded by the number of
  distinct shared formulas (tiny) and never by range size. Output is identical to
  calamine for every valid file.
- **Non-breaking.** `StreamValue::Formula` already exists and serializes through
  the TS surface (`index.d.ts`); this change only makes member cells populate it.
  No new public types, no API shape change.
- Closes GitHub issue #32 (and #25.2).
