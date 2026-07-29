## Why

When a worksheet merges a range (e.g. `F3:K3`) and styles the anchor cell with a border, Excel renders the border only under the anchor cell, not across the full merged range. Root cause: the writer **omits** non-anchor merged cells (`G3:K3`) from `<sheetData>`. Excel only extends a merged cell's border/formatting across the range when those cells physically exist in the grid. ExcelJS — this library's explicit drop-in compatibility target — always emits the full merge bounding box, so its files render correctly. Commit `1b8b28b` inverted the correctly-working approach by removing non-anchor cells, and the `merge-range-writer` spec now *mandates* the broken behavior.

## What Changes

- **Writer**: `write_cells_with_styles` in `src/writer/xlsx.rs` must **emit** every cell in a merged range's bounding box (anchor + non-anchors), each with its effective style — matching ExcelJS — instead of skipping non-anchor cells.
- **Spec inversion**: The `merge-range-writer` requirement "Writer filters non-anchor cells from merged ranges in sheetData" is removed and replaced by a requirement to emit the full merge bounding box.
- **Tests**: Update `test_merged_range_with_values_omitted` and `test_style_index_sync_after_merged_cells` (they assert the wrong behavior) and add a regression test asserting non-anchor cells ARE present in the emitted XML.
- No API or TS interface changes — fully backward-compatible.

## Capabilities

### New Capabilities

- None

### Modified Capabilities

- `merge-range-writer`: requirement changes from *omitting* non-anchor merged cells to *emitting the full merge bounding box* (so Excel renders the anchor's borders and formatting across the entire merged range, matching ExcelJS).

## Impact

- `src/writer/xlsx.rs`: `write_cells_with_styles` cell-emission loop changes; the style-index vector (`cell_style_indices`) must cover injected empty non-anchor cells (Normal, or column style when applicable) so the iterator stays aligned. ~40–80 line change.
- `src/model/worksheet.rs`: add a helper to enumerate the bounding-box cells of all merged ranges (or compute in the writer from `get_merged_ranges()`). `is_cell_merged_anchor` can remain.
- `openspec/specs/merge-range-writer/spec.md`: requirement inverted via delta (REM OVED + ADDED).
- Tests in `src/writer/xlsx.rs` + the spec delta.
