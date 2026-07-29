## Why

Column widths set via `setColumns()` and row heights set via `getRow().height` are silently lost when a worksheet is written to XLSX. The writer never emits the `width` attribute on `<col>` elements or the `ht` attribute on `<row>` elements, so Excel opens the file with default dimensions regardless of what was set.

## What Changes

- Fix `<col>` XML emission to include `width` attribute from `Column.width`
- Fix `<row>` XML emission to include `ht` attribute when `Row.height` is `Some`
- Emit `<cols>` for all columns that have an explicit width (not only grouped columns)
- Fix `getRow()` returning a clone so that `ws.getRow(n).height = x` persists to the stored Row

## Capabilities

### New Capabilities

- `dimension-properties`: Column widths and row heights survive a write/read round-trip; `<col width="...">` and `<row ht="...">` are emitted correctly in XLSX output

### Modified Capabilities

- `exceljs-parity`: `setColumns` width and `Row.height` now match exceljs behavior (dimensions persist in output)

## Impact

- `src/model/worksheet.rs`: `getRow()` and `addRow()` must return a handle that writes back to the stored Row instead of a detached clone
- `src/writer/xlsx.rs`: `emit_worksheet_cols` must emit `width` on `<col>` and include non-grouped columns with explicit widths; row XML emission must include `ht` attribute from `row.height()`
- `src/model/column.rs`: No changes needed (width is already stored)
- `src/model/row.rs`: No changes needed (height is already stored)
