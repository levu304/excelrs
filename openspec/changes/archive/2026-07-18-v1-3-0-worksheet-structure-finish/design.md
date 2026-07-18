# Design: v1.3.0 — Worksheet-structure parity finish

## Context

`excelrs` is a Rust (napi) native addon porting the ExcelJS API. v1.2.0 shipped
conditional formatting; the parity matrix still lists core worksheet-structure
areas as `planned`. The roadmap in `docs/spec.md` §9.4 pins v1.3.0 as
**Worksheet-structure parity finish** — inserting/splicing/duplicating rows,
row/column outline levels (grouping), and row/column page breaks — which
closes the remaining `planned` v1.x parity-matrix rows.

The codebase already proves every pattern these features need:

- **Per-feature `parse_sheet_*` reader passes** — `src/reader/xlsx.rs` builds the
  model from per-sheet XML via focused functions wired as `Step 3.x` in
  `workbook_inner_from_bytes` (e.g. `parse_sheet_data_validations`,
  `parse_sheet_conditional_formattings`, `parse_sheet_merge_cells`,
  `parse_sheet_row_styles`). New features slot in as new passes.
- **`Arc<Mutex<>>` interior mutability** on `Worksheet` `rows` / `columns` so any
  clone shares state across the FFI boundary (`src/model/worksheet.rs`).
- **`Row`/`Column`/`Cell` models** with getters/setters and serde camelCase
  (`src/model/row.rs`, `src/model/column.rs`, `src/model/cell.rs`).
- **`<row s="N">` emission + row-style read** already proven (v1.2.2 restored
  row-level styles), so the `<row>` element is already in the read/write path.

The change is **additive** — no existing API behavior changes; the minor bump
(`1.2.2` → `1.3.0`) signals new capability.

## Goals / Non-Goals

**Goals:**

- Deliver full read/write round-trip for row/column outline levels (grouping),
  row/column page breaks, and `insertRow` / `spliceRows` / `duplicateRow`.
- Renumber rows and cells correctly on any shift, preserving values and styles
  for rows outside the shifted range.
- Match Excel/ExcelJS on-disk layout exactly (element attributes + schema order).

**Non-Goals:**

- Column **width** / **hidden** round-trip. The writer emits no `<cols>` today
  and the reader does not model columns from file — that is a separate gap, not
  in this roadmap line. Column `outlineLevel` is emitted as a `<col outlineLevel>`
  attribute only when set; width/hidden are untouched.
- Row **height** / **hidden** emission. Same separate gap; not in scope.
- Grouping **collapse state** (`collapsed` / `ph`) beyond `outlineLevel`.
- Streaming XLSX — that is the v2.0.0 capstone.

## Decisions

### D1 — Unified row renumbering via an ordered `Vec<Row>`

`Worksheet.rows` is a `BTreeMap<u32, Row>` keyed by row number, and each `Row`
holds its own `number` plus `Cell`s that cache their `(row, col)` and A1
`address`. In-place shifting (insert/delete/copy) by mutating the map directly
risks overwrite collisions when moving a block of rows down.

**Decision:** implement all three mutations by collecting the rows into an
ordered `Vec<Row>`, applying the change at the right index (insert / `splice` /
copy), then re-keying the `BTreeMap` from the `Vec` with each `Row` renumbered
to its final 1-based position. `Row::renumber(new_number)` updates `self.number`
and calls `Cell::renumber(new_row)` on every cell — which updates the cell's
`row` field and recomputes its `address` via `Cell::compute_address`.

**Rationale:** mirrors ExcelJS's array-of-rows model exactly, guarantees
collision-free renumbering, and keeps a single reindex path for all three
operations (ponytail: one helper, not three). Collecting all rows is O(n),
acceptable for in-memory worksheet manipulation.

### D2 — `outlineLevel` is `u8` clamped to 0–7

Stored on `Row` as `Arc<Mutex<u8>>` (default 0) and on `Column` as a plain
`u8` serde field (default 0). The setter validates/clamps to the `0`–`7` range
(Excel's maximum nesting depth); values outside are clamped rather than rejected
to stay lenient like ExcelJS.

**Rationale:** matches the OOXML `outlineLevel` attribute and Excel's hard cap;
`Row` uses `Arc<Mutex<>>` to match the other `Row` fields' interior-mutability
model (so `row.outlineLevel = 2` persists through clones).

### D3 — Writer emits `<row>`/`<<col>` `outlineLevel` only when set

- **Rows:** in `write_cells_with_styles` (`src/writer/xlsx.rs`), emit
  `outlineLevel="N"` on `<row>` only when the row's `outlineLevel > 0`. Rows
  with level 0 keep today's output unchanged.
- **Columns:** the writer currently emits **no `<cols>` at all**. To avoid
  changing existing byte output, emit a minimal `<cols>` block **only when at
  least one column has `outlineLevel > 0`**, with one `<col min="N" max="N"
  outlineLevel="N"/>` per such column. When no column is grouped, no `<cols>`
  is written (identical to today).

**Rationale:** additive and byte-compatible for the default case; column
width/hidden attributes are intentionally omitted (Non-Goal D2 context).

### D4 — Page breaks as `BTreeSet<u32>` on `Worksheet`, emitted in schema order

Add `row_breaks: Arc<Mutex<BTreeSet<u32>>>` and `col_breaks: Arc<Mutex<BTreeSet<u32>>>`
to `Worksheet`. napi exposes `rowBreaks()` / `colBreaks()` (sorted `Vec<u32>`)
getters + `setRowBreaks` / `setColBreaks` setters.

- **Writer:** emit `<rowBreaks count="N">` and `<colBreaks count="N">` in the
  schema-correct position — after `sheetData` / `sheetProtection` / `autoFilter` /
  `mergeCells` / conditional formatting / `dataValidations` / `hyperlinks`, and
  **before** `pageMargins` (matches the current writer flow; insert between
  `hyperlinks` and `pageMargins`). Each break is `<brk id="R" max="16383"
  man="0"/>` for rows and `<brk id="C" max="1048575" man="0"/>` for columns.
  Omit both blocks entirely when empty.
- **Reader:** add `parse_sheet_row_breaks` / `parse_sheet_col_breaks` (mirror the
  existing `parse_sheet_*` pattern) scanning `<rowBreaks><brk id="…"/></rowBreaks>`
  / `<colBreaks>`, wired as Steps 3.17 / 3.18. Attach via internal
  `insert_row_break` / `insert_col_break`.

**Rationale:** correct OOXML element order is required for Excel to open the
file; the `id`/`max`/`man` triplet matches Excel-authored output.

### D5 — API shape for insert/splice/duplicate

- `Worksheet.insertRow(rowNumber: u32, values?: Vec<serde_json::Value>)` — insert
  a new empty row at `rowNumber`, shifting rows ≥ `rowNumber` down by 1; fill
  `values` if provided.
- `Worksheet.spliceRows(start: u32, count: u32, rows?: Vec<Vec<serde_json::Value>>)`
  — remove `count` rows at `start`, then insert the provided row-value arrays in
  their place (ExcelJS's variadic `...rows` maps 1:1 to this array).
- `Worksheet.duplicateRow(rowNumber: u32, count: u32, includeStyle: bool)` —
  insert `count` copies of the row immediately below `rowNumber`, copying the
  values and (when `includeStyle`) the row/cell styles.

**Rationale:** napi variadics are awkward, so `spliceRows` takes an explicit
`rows` array; the semantics match ExcelJS exactly. All three reuse the D1
ordered-`Vec` renumber.

### D6 — Round-trip fidelity acceptance bar

Every feature's correctness is proven by a read→write→read fixture, exercising
both ExcelJS-authored output (via the new API) and a real Excel-authored `.xlsx`
with grouping + manual page breaks. No feature ships without a fixture — mirroring
the approach used by `images`, `hyperlinks`, `tables`, and conditional formatting.

### D7 — Minimal substrate, no new dependencies

Reuses `quick-xml`, `napi`, `BTreeSet`, and the existing `Style` primitives.
No new crates. No column width/hidden/height plumbing (Non-Goals).

## Trade-offs

- **[Risk] Renumber correctness on overlapping shifts** → D1's collect-then-rekey
  on an ordered `Vec` makes collisions impossible; `Row::renumber` +
  `Cell::renumber` keep `address` consistent with `row`.
- **[Risk] Byte drift for files with no grouping/breaks** → D3/D4 emit new XML
  **only when the feature is actually used**, so default output is unchanged.
- **[Risk] Reader schema gaps on Excel-authored grouping/breaks** → D6 fixtures
  include real Excel-authored `.xlsx`, not just excelrs output, to catch gaps.

## Migration Plan

1. Model: add `Row.outlineLevel` + `Row::renumber`; add `Column.outlineLevel`;
   add `Cell::renumber`; add `Worksheet.row_breaks` / `col_breaks` +
   `insertRow` / `spliceRows` / `duplicateRow` + reader hooks.
2. napi bridge: expose the new `Row` / `Column` / `Worksheet` members in
   `src/lib.rs` + `index.d.ts`.
3. Reader: add the four `parse_sheet_*` passes (row/col outline levels, row/col
   breaks) wired as Steps 3.17–3.20.
4. Writer: emit `<row outlineLevel>`, minimal `<cols>` for grouped columns, and
   `<rowBreaks>` / `<colBreaks>` in correct position.
5. Fixtures: ExcelJS-authored round-trip for each feature + Excel-authored
   grouping/breaks `.xlsx`; assert survival on write→read.
6. Release: bump `package.json` `1.2.2` → `1.3.0`; update `CHANGELOG.md` and
   `ROADMAP.md` parity matrix (remaining v1.x rows → `shipped`); sync the
   `exceljs-parity` spec delta; archive the change when fixtures pass.

## Open Questions

- **`spliceRows` `rows` param shape** — confirm the array-of-arrays `Vec<Vec<Value>>`
  is preferable to a variadic for the TS surface (decided: array, per D5).
- **`duplicateRow` placement** — copies go immediately below the source row
  (matches ExcelJS); confirm that is the desired default.
