## Why

When cells in a merged range have data in every column (e.g., row 3 filled via `addRow` then `mergeCells("F3:K3")`), the writer emits every cell individually with `s="0"` (Normal, no border). Excel sees conflicting instructions — the `<mergeCell>` says merge F3:K3, but G3:K3 each say "no border" via their `s="0"` style reference — and renders only the top-left cell's border, breaking the expected merged-cell border appearance.

## What Changes

- **Writer**: `write_cells_with_styles` in `src/writer/xlsx.rs` must skip cells whose address falls inside any merged range, except the top-left anchor cell of that range.
- **Reader**: `get_cell_by_address` for merged-range non-anchor cells continues to return the cell (no API change). Only the XML emission path changes.
- **No API or TS interface changes** — fully backward-compatible.

## Capabilities

### New Capabilities

- `merge-range-writer`: Filter merged-range shadow cells from `sheetData` XML emission

### Modified Capabilities

- None

## Impact

- **`src/writer/xlsx.rs`**: `write_cells_with_styles` gets a merge-range filter pass. 40–60 line change.
- **`src/model/worksheet.rs`**: Add a helper to check if a (row, col) is a non-anchor cell in a merged range. 10–15 line addition.
- **Existing tests update**: `test_normal_cell_has_s_attr` (uses row data that may overlap merges — verify), plus new tests added in earlier exploration.
- **No perf concern**: merge ranges are small, scan is O(ranges) per cell on a typically tiny list.
