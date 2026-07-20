## 1. Port reference translation from calamine

- [x] 1.1 Port `Reference` enum (`Cell` / `Row` / `Column`) with `parse` / `offset` / `format` into `src/stream.rs`, adapted to `ExcelrsError` (return `Read` variant on overflow/parse failure) and the workbook bounds constants (`MAX_COLUMNS` = 16384, `MAX_ROWS` = 1048576).
- [x] 1.2 Port `offset_range` (splits on `:`, parses each side, applies `offset`, re-formats) and `replace_cell_names` (tokenizes the formula, skips tokens before `(` and inside `"..."`, copies verbatim anything that fails to parse).
- [x] 1.3 Port `column_number_to_name` helper; confirm it matches calamine's output byte-for-byte for the column range used by tests.

## 2. Wire shared-formula table into `parse_sheet_rows`

- [x] 2.1 Add a per-sheet `HashMap<u32 /*si*/, (String /*master text*/, (u32,u32) /*master pos*/)>` to `parse_sheet_rows` and reset it per sheet call (it is already called once per worksheet).
- [x] 2.2 Extend the `<f>` start/empty handlers to read the `<f>` element's `t`, `si`, and `ref` attributes (currently none are read).
- [x] 2.3 On a master `<f>` (has `ref` + inline text): insert `(text, master_pos)` under `si`; leave `formula_buf` populated so the master emits `Formula(text)` (offset 0).
- [x] 2.4 On a member `<f>` (`t="shared"`, no inline text): look up `si`; if found, compute `offset = (member_pos - master_pos)` and write `replace_cell_names(master_text, offset)` into `formula_buf` (with `has_formula` already `true`) so `build_cell_value` returns `StreamValue::Formula(translated)`; if not found, leave it unresolved (no formula).
- [x] 2.5 Confirm the existing cell-boundary formula-reset guard and the `MAX_EVENTS` / `MAX_ENTRY_BYTES` contract are unaffected.

## 3. Spec & docs

- [x] 3.1 Add `specs/streaming-xlsx/spec.md` ADDED requirement "Streaming reader resolves shared formulas" with the member / master / absolute / non-shared / memory-bounds / malformed scenarios.
- [x] 3.2 Update ROADMAP shared-formula row (was "known limitation / deferred") to reflect resolution; close GitHub issues #32 and #25.2.

## 4. Tests

- [x] 4.1 Add a fixture `.xlsx` with a shared formula (`=A1+B1` master at `B2`, `si="0"`, `ref="B2:B10"`) plus a non-shared formula and a sheet-qualified (`Sheet1!A1`) reference; verify the file is valid via the whole-workbook reader.
- [x] 4.2 Rust unit/integration test: stream-read the fixture and assert `B5` resolves to `=A4+B4`, `B2` (master) to `=A1+B1`, the non-shared formula is unchanged, and an absolute ref in a shared formula is preserved.
- [x] 4.3 Fidelity test: assert the streaming reader's resolved formulas equal the whole-workbook (calamine) reader's output for the same fixture (or call calamine directly in the test) — covers the "same as whole-workbook reader" acceptance.
- [x] 4.4 Negative test: a member cell whose `si` is never defined emits no `Formula` and does not panic.
- [x] 4.5 Memory/resource check: streaming a sheet with shared formulas still honors `MAX_ENTRY_BYTES` / `MAX_EVENTS` (no new unbounded allocation).
