## 1. Resolver (reader/xlsx.rs)

- [ ] 1.1 Add `pub fn resolve_sheet_paths(data: &[u8], sheet_count: usize) -> Vec<String>` that reads `xl/workbook.xml.rels` (rId→target) and the `<sheet r:id>` order from `xl/workbook.xml`, resolves targets relative to `xl/`, and falls back to positional `sheet{i+1}.xml` when rels/rId are absent.
- [ ] 1.2 Build the rId→target map reusing the existing `parse_sheet_rels` HashMap style (match `e.name().as_ref() == b"Relationship"`, `attr.key.as_ref()` for `Id`/`Target`).
- [ ] 1.3 Add private helpers `sheet_rels_path(sheet_path: &str) -> String` and `fallback_sheet_paths(sheet_count: usize) -> Vec<String>`.

## 2. Thread through bulk parsers (reader/xlsx.rs)

- [ ] 2.1 In `workbook_inner_from_bytes`, compute `let sheet_paths = resolve_sheet_paths(data, sheet_count);` once after `sheet_count` is known.
- [ ] 2.2 Convert the ~20 bulk per-sheet parsers' signature `sheet_count: usize` → `sheet_paths: &[String]`.
- [ ] 2.3 Replace each `format!("xl/worksheets/sheet{}.xml", i + 1)` / `, sheet_num` with `sheet_paths[i]` (`.clone()`). Replace `xl/worksheets/_rels/sheet{}.xml.rels` (`sheet_num`) with `sheet_rels_path(&sheet_paths[i])` and drop the now-unused `let sheet_num = i + 1;`.
- [ ] 2.4 Pass `&sheet_paths` at the ~20 bulk call sites in `workbook_inner_from_bytes`. Keep the `styles::parse_styles_and_sheet_maps(data, sheet_count)` call unchanged (it self-resolves).

## 3. Shared cell-style parser (reader/styles.rs)

- [ ] 3.1 Keep `parse_styles_and_sheet_maps(data, sheet_count: usize)` signature (also used by the streaming reader). Inside, compute `let sheet_paths = crate::reader::xlsx::resolve_sheet_paths(data, sheet_count);` and repath its single `xl/worksheets/sheetN.xml` read. Fixes bulk-read cell-style alignment without breaking the streaming reader or its tests.

## 4. Cleanup

- [ ] 4.1 Remove the obsolete `ponytail:` deferral note at `src/reader/xlsx.rs:~2360` (now obsolete).
- [ ] 4.2 Confirm `parse_sheet_states(data, &sheet_names)` (display-order, reads `xl/workbook.xml`, no file read) needs no change.
- [ ] 4.3 Update the stale header comment at `src/reader/xlsx.rs:~2356` (file-index assumption) to point at the resolver.

## 5. Tests

- [ ] 5.1 Add a regression test that builds a reordered workbook (display A,B,C; files sheet1=A, sheet3=B, sheet2=C) with distinct `tabColor` per sheet, and asserts each `inner.worksheets[i].tab_color` lands on the correct worksheet.
- [ ] 5.2 Run `cargo test --lib` round-trip suite to confirm no regression.
