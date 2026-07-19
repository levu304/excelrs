## 1. Fix absolute rels Target path (#28)

- [x] 1.1 In `parse_workbook_sheet_targets` (src/stream.rs), resolve the sheet path with a branch: an absolute rels `Target` (leading `/`) maps to the package path with the `/` stripped; a relative `Target` keeps the `format!("xl/{}", target)` form.
- [x] 1.2 Add a unit test that builds an xlsx whose `xl/_rels/workbook.xml.rels` uses `Target="/xl/worksheets/sheet1.xml"` (sheet entry physically at `xl/worksheets/sheet1.xml`) and asserts the sheet is read (non-empty), proving the doubled-`xl/` bug is fixed.

## 2. Fix MAX_EVENTS doc comment (#30)

- [x] 2.1 Move the "Max SAX events per sheet (anti-billion-row / entity-expansion guard)." `///` line out of the `MAX_ENTRY_BYTES` doc block to a `///` directly above `const MAX_EVENTS` (src/stream.rs).

## 3. Verify

- [x] 3.1 Run `cargo test --lib stream` and `cargo clippy -- -D warnings` and confirm green.
