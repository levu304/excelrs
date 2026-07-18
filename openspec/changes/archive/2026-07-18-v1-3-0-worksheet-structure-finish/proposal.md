# Proposal: v1.3.0 — Worksheet-structure parity finish

## Why

`excelrs` reached the v1.2.0 conditional-formatting milestone, but the parity
matrix still lists core **worksheet-structure** areas as `planned`: row/column
insertion & splicing (`insertRow` / `spliceRows` / `duplicateRow`), row/column
**outline levels** (grouping), and row/column **page breaks** (`rowBreaks` /
`colBreaks`). These are bread-and-butter ExcelJS worksheet APIs
(`ws.insertRow`, `ws.spliceRows`, `ws.duplicateRow`, `Row.outlineLevel`,
`Column.outlineLevel`, `ws.rowBreaks`, `ws.colBreaks`). Today they do not exist
on `excelrs` — so any workbook that uses grouping or manual page breaks loses
that structure on round-trip, and code that calls the insert/splice/duplicate
APIs fails outright. That breaks the drop-in compatibility promise for basic
worksheet manipulation.

The roadmap in `docs/spec.md` §9.4 pins this exactly:

> | **v1.3.0** | **Worksheet-structure parity finish** | medium | targeted |
> `insertRow(s)` / `spliceRows` / `duplicateRow`; row/col `outlineLevel`
> (grouping); `rowBreaks` / `colBreaks` page breaks — closes the remaining
> "planned" v1.x parity-matrix rows |

v1.3.0 closes those remaining `planned` v1.x parity-matrix rows, leaving only
the v2.0.0 streaming capstone as the unshipped matrix area.

## What Changes

Add three worksheet-structure capabilities with full read/write round-trip:

- **Row/Column outline level (grouping)**: add `outlineLevel` (valid `0`–`7`,
  Excel's cap) to `Row` and `Column`. `Worksheet` emits `<row outlineLevel="N">`
  and `<col outlineLevel="N">`; the reader restores them on read so grouping
  survives the round-trip.
- **Page breaks**: add `rowBreaks` / `colBreaks` (a set of 1-indexed numbers) to
  `Worksheet`. `Worksheet` emits `<rowBreaks>` / `<colBreaks>` in schema-correct
  position; the reader parses them back.
- **Row insertion / mutation**: add `insertRow(rowNumber, values?)`,
  `spliceRows(start, count, rows?)`, and `duplicateRow(rowNumber, count,
  includeStyle)` to `Worksheet`. Shifting rows renumbers each affected `Row`
  and every `Cell` inside it (row number + A1 address), preserving values and
  styles — without disturbing rows outside the shifted range.

- **BREAKING**: none — strictly additive. Minor version bump
  (`1.2.2` → `1.3.0`) signals new capability; no existing API behavior changes.

## Capabilities

### New Capabilities

- `worksheet-structure`: Worksheet-structure parity — `Row.outlineLevel` /
  `Column.outlineLevel` grouping (read/write round-trip of `<row>`/`<col>`
  `outlineLevel`), `Worksheet.rowBreaks` / `Worksheet.colBreaks` page breaks
  (read/write round-trip of `<rowBreaks>`/`<colBreaks>`), and `Worksheet`
  `insertRow` / `spliceRows` / `duplicateRow` with correct row + cell
  renumbering.

### Modified Capabilities

- `exceljs-parity`: v1.3.0 advances the remaining v1.x `planned` matrix rows
  (`insert/splice/duplicate rows`, `row/col outlineLevel (grouping)`,
  `row/col page breaks`) → `shipped`. The release-recording requirement gains a
  v1.3.0 scenario.

## Impact

- **Code**:
  - `src/model/row.rs` — add `outlineLevel` (`Arc<Mutex<u8>>`, 0–7) + getter/setter; add `Row::renumber(new_number)` (updates `number` + each `Cell` row/address).
  - `src/model/column.rs` — add serde `outlineLevel: u8` field + getter/setter.
  - `src/model/cell.rs` — add private `Cell::renumber(new_row)` updating `row` + `address` (used by row shifting).
  - `src/model/worksheet.rs` — add `row_breaks` / `col_breaks` (`Arc<Mutex<BTreeSet<u32>>>`), `insertRow` / `spliceRows` / `duplicateRow` methods (unified ordered-`Vec<Row>` renumber), and `insert_row_outline_level` / `insert_column_outline_level` / `insert_row_break` / `insert_col_break` reader hooks.
  - `src/reader/xlsx.rs` — new `parse_sheet_row_outline_levels`, `parse_sheet_col_outline_levels`, `parse_sheet_row_breaks`, `parse_sheet_col_breaks` (mirror existing `parse_sheet_*` passes; wired as Steps 3.17–3.20).
  - `src/writer/xlsx.rs` — emit `outlineLevel` on `<row>`; emit minimal `<cols>` only when a column has `outlineLevel > 0`; emit `<rowBreaks>` / `<colBreaks>` (in schema-correct position, after `sheetData`/merge/CF/DV/hyperlinks, before `pageMargins`).
  - `src/lib.rs` + `index.d.ts` — expose new types/methods on `Worksheet` / `Row` / `Column`.
- **APIs**: additive napi surface on `Worksheet`, `Row`, `Column`. No changes to existing public types.
- **Dependencies**: none added; reuses `quick-xml`, `napi`, and the existing `Style` primitives.
- **Specs**: new `worksheet-structure` capability spec; advances `exceljs-parity` matrix rows → `shipped`.
- **Parity domain**: closes the last remaining "planned" v1.x parity-matrix rows, leaving only the v2.0.0 streaming capstone unshipped.
