## 1. Model Helper

- [x] 1.1 Add `fn is_cell_merged_anchor(&self, row: u32, col: u32) -> bool` to `src/model/worksheet.rs`
- [x] 1.2 Tests for `is_cell_merged_anchor` — anchor cell, non-anchor inside range, outside range

## 2. Writer Filter

- [x] 2.1 In `write_cells_with_styles` (`src/writer/xlsx.rs`), parse merge ranges into `Vec<(u32, u32, u32, u32)>` once before the cell loop
- [x] 2.2 Add filter in the cell-write loop: skip cells where cell is in a merged range but not anchor
- [x] 2.3 Verify existing tests pass: `test_normal_cell_has_s_attr`, `test_round_trip_merge_cells`, `test_merge_cells_with_border_roundtrip`, `test_merge_cells_border_only_xml`

## 3. Verification

- [x] 3.1 Run full test suite: `cargo test`
- [x] 3.2 Verify border renders across merged range (XML check + ExcelJS read-back confirms G3:K3 excluded from sheetData)
- [x] 3.3 `test_normal_cell_has_s_attr` passes as-is — no row overlap with merge ranges in that test
