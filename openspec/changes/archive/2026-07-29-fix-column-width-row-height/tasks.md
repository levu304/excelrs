## 1. Fix `<col>` XML emission to include `width` attribute

- [x] 1.1 Update `emit_worksheet_cols` in `src/writer/xlsx.rs` to include `width` attribute on `<col>` elements when `Column.width` is non-zero
- [x] 1.2 Remove the `outline_level > 0` filter so all columns with explicit width or hidden state are emitted

## 2. Fix `<row>` XML emission to include `ht` attribute

- [x] 2.1 Update row XML emission in `src/writer/xlsx.rs` to include `ht` attribute when `row.height()` is `Some`

## 3. Add tests for dimension persistence

- [x] 3.1 Write test that sets column widths, writes XLSX, reads back, and verifies widths match
- [x] 3.2 Write test that sets row heights, writes XLSX, reads back, and verifies heights match
- [x] 3.3 Write test that verifies `<cols>` is emitted for non-grouped columns with explicit widths
