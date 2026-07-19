## 1. Redundant zip I/O removal (A2)

- [x] 1.1 Change `parse_workbook_sheet_targets` to accept `&mut ZipArchive<Cursor<&[u8]>>` instead of `&[u8]`; use the passed archive for the `workbook.xml.rels` and `workbook.xml` reads (stream.rs ~L142).
- [x] 1.2 Change `parse_shared_strings` to accept `&mut ZipArchive<Cursor<&[u8]>>`; use the passed archive instead of opening its own (stream.rs ~L239).
- [x] 1.3 Update `stream_read` to open the archive once and pass `&mut archive` to both helpers (stream.rs ~L106/L117/L133).
- [x] 1.4 Confirm no behavior change: existing `stream_read_preserves_*` / round-trip tests still pass.

## 2. Sheet file resolution (A1)

- [x] 2.1 Change `parse_workbook_sheet_targets` return type from `Vec<(String, u32)>` to `Vec<(String, String)>` where the string is the `xl/`-prefixed rels target path; preserve the `Sheet1` fallback path.
- [x] 2.2 Remove `sheet_number_from_target` (stream.rs ~L231).
- [x] 2.3 Update the `stream_read` loop to `archive.by_name(&path)` directly instead of `format!("xl/worksheets/sheet{}.xml", num)` (stream.rs ~L117).
- [x] 2.4 Add test: a workbook whose sheet file is a non-default name (e.g. `worksheets/sheet_v2.xml`) is read from the correct file with correct sheet order.

## 3. Hostile-input size guard + cap consistency (A3, #25.3)

- [x] 3.1 At each streamed entry read (workbook.xml, rels, sharedStrings.xml, per-sheet), replace `read_to_string` with `entry.take(MAX_ENTRY_BYTES).read_to_string(&mut s)?` while keeping the declared-size friendly error (stream.rs L121/L147/L183/L244 + per-sheet read ~L121).
- [x] 3.2 Document `MAX_ENTRY_BYTES` (16 MiB) and `MAX_EVENTS` (5,000,000) as the streaming resource contract in a comment near the constants (stream.rs ~L93).
- [x] 3.3 Add test: a zip entry declaring a small size but decompressing past the cap returns an error (no OOM); a legitimately oversized part returns the clear "exceeds streaming size limit" error.

## 4. Empty-cell fidelity (A5)

- [x] 4.1 Add `Empty` variant to `StreamValue` (stream.rs ~L41).
- [x] 4.2 In `stream_handle.rs`, add additive `empty: Option<bool>` to `JsStreamValue`; `from_js_value` returns `Empty` when no field is set; `to_js_value` emits `empty: Some(true)` for `Empty`.
- [x] 4.3 Handle `Empty` in the writer (emit a cell with no `<v>`) and `build_cell_value` so it round-trips as empty.
- [x] 4.4 Add test: empty JS cell (`{}` / `{ value: null }`) round-trips as empty (not `Text("")`); `""` round-trips as a text cell.

## 5. Formula-capture regression (A4)

- [x] 5.1 Add regression test feeding sheet XML where `<f>` opens but `</f>` is missing before `</c>`; assert the next cell's value is its own (not appended to the prior formula). No code change — verifies the existing v2.0.0 fix.

## 6. Verification

- [x] 6.1 `cargo build` and `cargo test` pass; run the streaming smoke / round-trip checks.
- [x] 6.2 `openspec validate v2-1-0-streaming-hardening` passes (proposal/design/specs/tasks consistent).
