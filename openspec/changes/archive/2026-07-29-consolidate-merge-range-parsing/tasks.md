## 1. Extract Shared Helper

- [x] 1.1 Add `fn parse_merge_range(&self, range: &str) -> Option<(u32, u32, u32, u32)>` to `src/model/worksheet.rs` — splits on `:`, parses both endpoints via `parse_address`, returns `(anchor_col, anchor_row, end_col, end_row)`
- [x] 1.2 Refactor `is_merged()` to call `self.parse_merge_range()` instead of duplicating split+parse logic
- [x] 1.3 Refactor `is_cell_merged_anchor()` to call `self.parse_merge_range()` instead of duplicating split+parse logic

## 2. Wire Writer to Use Helper

- [x] 2.1 Replace inline merge filter in `write_cells_with_styles` (`src/writer/xlsx.rs`) with call to `ws.is_cell_merged_anchor(cell_row, cell_col)`
- [x] 2.2 Remove pre-parsed `merged_ranges_parsed` vector from `write_cells_with_styles` (no longer needed)

## 3. Fix Style-Index Desync

- [x] 3.1 Move `cell_si.next()` before the merge-range skip check so iterator advances for every cell in `written_cells()`, even skipped ones

## 4. Add TDD Regression Test

- [x] 4.1 Write test: non-anchor merged cell WITH value and style is suppressed in XML output (the exact regression scenario from the original bug report)
- [x] 4.2 Write test: style iterator stays in sync when non-anchor cells are skipped (cells after merge range get correct `s` attribute)

## 5. Verify

- [x] 5.1 Run `cargo test` — all tests pass (395/395)
- [x] 5.2 Run `cargo clippy` — no new warnings
- [x] 5.3 Verify XML output: non-anchor cells with values absent from sheetData, anchor cell has correct style, cells after merge range have correct style index
