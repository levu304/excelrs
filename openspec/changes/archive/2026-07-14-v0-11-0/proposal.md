## Why

`excelrs` is a drop-in Rust/Node ExcelJS replacement and the parity matrix (ROADMAP.md) still marks four high-visibility worksheet features as `planned`: hyperlinks (read), auto filters, freeze/split panes, and sheet protection. Each is a single, self-contained OOXML element with `low` effort and `high` compat value — the textbook "quick-win" tier. Shipping them in one release closes the most noticeable round-trip gaps for ExcelJS users at minimal cost.

## What Changes

- **Hyperlinks (read)**: Parse `<hyperlinks>` from sheet XML so `cell.value = { text, hyperlink }` round-trips after a read-modify-write cycle. Write-side hyperlinks already ship (v0.5.0); this change adds only the reader half.
- **Auto filters**: Read and write the single `<autoFilter ref="…">` attribute on a worksheet. Exposes `ws.autoFilter = "A1:C1"` parity.
- **Freeze panes / split views**: Read and write `<sheetViews><sheetView>` pane state (`state="frozen"`, `xSplit`, `ySplit`, `topLeftCell`, `activePane`). Exposes `worksheet.views = [{ state, xSplit, ySplit }]` parity.
- **Sheet protection**: Read and write `<sheetProtection>` boolean flags. Exposes `ws.protection = { locked, ... }` parity.

No breaking changes. All four are additive reader/writer element support.

## Capabilities

### New Capabilities

- `hyperlinks`: Worksheet hyperlink read (and write round-trip) coverage, keyed off `<hyperlinks>` in sheet XML.
- `auto-filter`: Worksheet auto-filter attribute read/write (`ws.autoFilter`).
- `worksheet-views`: Freeze/split pane state read/write (`worksheet.views`, `<sheetView>`/`<pane>`).
- `sheet-protection`: Sheet protection flags read/write (`ws.protection`, `<sheetProtection>`).

### Modified Capabilities

- `exceljs-parity`: Parity matrix status advances from `planned` → `shipped` for hyperlinks, auto-filter, freeze panes, and sheet protection on release of v0.11.0.

## Impact

- **Reader** (`src/reader/`): new parsing for `<hyperlinks>`, `<autoFilter>`, `<sheetViews>/<sheetView>/<pane>`, `<sheetProtection>` in `xl/worksheets/sheetN.xml`.
- **Writer** (`src/writer/`): new emission of the same elements; ordering must follow existing sheet-XML sequence (e.g. `<autoFilter>` sits between `<mergeCells>` and `<dataValidations>`).
- **Model** (`src/model/`): new fields on `Worksheet` (autoFilter, views, protection) and `Cell`/`CellValue` (hyperlink variant on read).
- **FFI** (`src/lib.rs`, `index.d.ts`, `native.d.ts`): surface new worksheet properties and the hyperlink value shape to JS.
- **Specs**: four new capability specs; `exceljs-parity` matrix status update.
- **No new dependencies**; all four are pure OOXML element handling already partially present (hyperlinks write, data-validation adjacency).
