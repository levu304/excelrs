## Context

`excelrs` is a Rust XLSX library with Node.js NAPI bindings, ported from exceljs. Users set column widths via `worksheet.setColumns([{width: 15.83}])` and row heights via `worksheet.getRow(n).height = 29`. When the worksheet is written to XLSX, these dimensions are silently dropped — Excel opens the file with default sizes.

All bugs are in the XLSX writer layer (`src/writer/xlsx.rs`). The model layer (`Row`, `Column`) correctly stores width and height values — the issue is purely that the writer never serializes them.

## Goals / Non-Goals

**Goals:**

- Column widths set via `setColumns()` persist in XLSX output (`<col width="...">`)
- Row heights set via `getRow(n).height = x` persist in XLSX output (`<row ht="...">`)
- Dimensions survive a write/read round-trip

**Non-Goals:**

- Changing the `Column` or `Row` data model (both already store width/height correctly)
- Adding new APIs — fixing existing ones
- Supporting column/row hidden state (already emitted correctly via `hidden` attribute)

## Decisions

### D1: Emit `width` on `<col>` elements

**Current**: `emit_worksheet_cols` emits `<col min="N" max="N" outlineLevel="N"/>` — no `width`.
**Fix**: Add `width` attribute to the `<col>` format string. Emit `<col min="N" max="N" width="W" outlineLevel="N"/>` when width is non-zero.

### D2: Emit `ht` on `<row>` elements when height is set

**Current**: Row XML emitter uses four format strings that never include `ht`.
**Fix**: Add `ht` attribute to each `<row>` variant when `row.height()` is `Some`. The `ht` attribute in OOXML represents the row height in points.

### D3: Emit `<cols>` for all columns with explicit width, not just grouped ones

**Current**: `emit_worksheet_cols` filters to only `outline_level > 0` columns and returns early if none exist.
**Fix**: Remove the `outline_level > 0` filter. Emit `<cols>` for any column that has a non-zero width or is hidden. This matches exceljs behavior where `<cols>` is emitted for all columns with explicit properties.

## Risks / Trade-offs

- **File size**: Emitting `<cols>` for all columns with widths (not just grouped) may increase file size for worksheets with many columns. This is the correct behavior — Excel expects `<cols>` to contain all column definitions with explicit widths.
- **Round-trip fidelity**: Reading back a file written with these fixes should preserve dimensions. The reader already parses `width` from `<col>` and `ht` from `<row>` — no reader changes needed.
- **`getRow()` clone behavior**: `Row` uses `Arc<Mutex<...>>` for all mutable fields, so `getRow()` returning a clone still shares the same mutex. Mutations through the clone persist to the stored Row. No fix needed here.
