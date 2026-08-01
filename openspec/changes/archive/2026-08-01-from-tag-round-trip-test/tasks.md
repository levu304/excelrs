- [x] 1.1 Add `test_cell_type_from_tag_all_tags` in `src/model/cell.rs` `mod tests` — assert each of the 10 tag strings (`"Null"`, `"Number"`, `"String"`, `"Boolean"`, `"Date"`, `"Formula"`, `"Error"`, `"Hyperlink"`, `"RichText"`, `"Merge"`) maps to the correct `CellType` via `CellType::from_tag`
- [x] 1.2 Assert `from_tag("Unknown") == None` and `from_tag("")` == None` in the same test
- [x] 1.3 Add a comment noting the test array must be updated when a new `CellType` variant is added (no compile-time exhaustiveness since `from_tag` has a `_` arm)

## 2. Round-trip test via CellValue constructors

- [x] 2.1 Add `test_cell_type_from_tag_round_trip_via_cell_value` — for each `CellValue` constructor (`default`, `number`, `string`, `boolean`, `formula`, `hyperlink`, `rich_text`, `date`), construct a `CellValue`, extract `.value_type`, assert `from_tag` parses it back to the matching `CellType`
- [x] 2.2 This covers 8 of 10 variants (Null, Number, String, Boolean, Date, Formula, Hyperlink, RichText). The remaining 2 (`Error`, `Merge`) have no Rust constructors and are covered by the direct test in 1.1

## 3. Verify

- [x] 3.1 `cargo test --lib cell::tests` — all tests pass, new tests included
- [x] 3.2 `cargo clippy -- -D warnings` — clean (no new warnings)
