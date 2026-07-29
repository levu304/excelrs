## 1. Model Helper

- [x] 1.1 Add a helper on `Worksheet` (e.g. `merged_bounding_box_cells()` or compute inline in the writer) that returns all non-anchor addresses of every merged range whose cells are not already in `written_cells()`.
- [x] 1.2 Unit-test the helper: anchor-only merge (`B2:D4` with value only on `B2` → yields `C2,D2,B3,C3,D3,B4,C4,D4`), multi-range, no-merge cases.

## 2. Writer — Expand Cell Set

- [x] 2.1 In the top-level style build loop (`workbook_to_bytes`, `src/writer/xlsx.rs`), build the per-sheet cell+style list from `written_cells()` PLUS the injected empty non-anchor merged addresses; compute each injected cell's effective style (column style at that column if present, else `None`/Normal). Push into `cell_styles` and count toward `cell_count` so `cell_style_indices` covers them.
- [x] 2.2 In `write_cells_with_styles`, remove the `is_merged && !is_cell_merged_anchor` skip filter. Merge the injected empty non-anchor addresses into the per-row emission sequence (sorted by column, no `<v>`), emitting `<c r="ADDR" s="{style_idx}"/>`.
- [x] 2.3 Verify the `cell_si` iterator stays aligned: cells after a merge range (e.g. `L3`) keep the correct style index.

## 3. Spec Delta

- [x] 3.1 Create `specs/merge-range-writer/spec.md` delta: REMOVED "Writer filters non-anchor cells from merged ranges in sheetData" and ADDED "Writer emits merged range bounding box in sheetData" with the five scenarios.

## 4. Tests

- [x] 4.1 Rewrite `test_merged_range_with_values_omitted` to assert non-anchor cells `G3`..`K3` ARE present in the emitted XML (and `F3` keeps its thick-border style index).
- [x] 4.2 Rewrite `test_style_index_sync_after_merged_cells` to assert non-anchors are present AND the post-merge cell (`L3`) still receives the correct style index.
- [x] 4.3 Add regression test: generate a `F3:K3` merge with a thick bottom border on `F3`, unzip/inspect sheet XML, assert `G3`..`K3` cells exist; optionally compare the cell set against equivalent ExcelJS output.
- [x] 4.4 Run `cargo test` — full suite passes.

## 5. Verification

- [x] 5.1 Reproduce the user's scenario (three merges `F1:K1`/`F2:K2`/`F3:K3` + border on `F3` + borders on `A3`..`D3`), write the file, unzip, confirm all merge non-anchor cells are present in `<sheetData>`.
- [x] 5.2 Round-trip read-back: merged ranges and anchor border survive; non-anchor cells re-read without error.
