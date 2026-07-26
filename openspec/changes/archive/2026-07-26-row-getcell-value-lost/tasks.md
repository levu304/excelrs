# Tasks: Row.getCell() value mutations lost on cloned rows

TDD cycle: Red (write failing test) → Green (make changes) → Verify tests pass.

## 1. Red: Write failing test for the bug

- [x] 1.1 Add test `test_row_getcell_orphan` in `src/model/row.rs`:
  - Create Row::new(1)
  - Clone it (simulates `getRow` → napi clone)
  - Call `get_cell_by_col_letter("A")` on the clone
  - Set value on returned Cell via `set_value_raw`
  - Assert the **original** Row has the cell with the value
  - This test should **fail** against current code (cell only exists in clone's HashMap)

- [x] 1.2 Add integration-style test in `src/model/worksheet.rs`:
  - Create Worksheet
  - `get_row(5)`, `get_cell_by_col_letter("A")`, set value
  - Re-acquire cell from worksheet's `get_cell_by_rc` — assert value persisted
  - Existing `test_missing_row_getcell_persists` covers `ws.get_cell_by_rc` path but not `row.getCell` path. Add a test named `test_row_getcell_via_getrow_persists_value` that goes through `ws.get_row(1).get_cell_by_col_letter(...)`.

## 2. Green: Change Row struct to use Arc<Mutex<HashMap>>

- [x] 2.1 Change field declaration in `row.rs`:
  - `cells: HashMap<u32, Cell>` → `cells: Arc<Mutex<HashMap<u32, Cell>>>`
  - Update `# [derive(Clone)]` — `Arc` auto-implements Clone correctly, no manual work needed

- [x] 2.2 Update `Row::new`: wrap cells in `Arc::new(Mutex::new(HashMap::new()))`
  - Update `default()` if required (check if `Row: Default` exists)

- [x] 2.3 Rewrite `get_or_create_cell_mut`:
  - Rename to `get_or_create_cell`
  - Signature: `pub fn get_or_create_cell(&self, col: u32) -> Cell`
  - Body: lock the Mutex, entry + or_insert_with, clone the Cell
  - Remove the `number` capture (now in the closure via `self.number`)

- [x] 2.4 Update `set_cell_value`: change `&mut self` → `&self`, delegate to `get_or_create_cell`

- [x] 2.5 Update `get_cell_by_col_num`: change `&mut self` → `&self`, delegate to `get_or_create_cell`

- [x] 2.6 Update `get_cell_by_col_letter`: already delegates to `get_cell_by_col_num` — will pick up the `&self` via signature change

- [x] 2.7 Update `cell_count`: `.cells.lock().unwrap().len()`
- [x] 2.8 Update `max_col`: `.cells.lock().unwrap().keys().max()`
- [x] 2.9 Update `sorted_cells`: lock, collect sorted, return `Vec<Cell>` (was `Vec<&Cell>`)
- [x] 2.10 Update `written_cells`: adapt filter for owned `Cell` — `cell.is_effectively_empty()` still works on `&Cell`
- [x] 2.11 Update `detach_styles`: lock, deep-clone HashMap entries, replace Arc
- [x] 2.12 Update `clear_styles`: lock, iterate `values_mut()`
- [x] 2.13 Update `renumber`: lock, iterate `values_mut()`

- [x] 2.14 Run `cargo build` — fix any compile errors in reader/writer callers

## 3. Verify: All callers outside Row compile

- [x] 3.1 Check `reader/xlsx.rs` for `get_or_create_cell_mut` callers:
  - Reader calls `ws_row.get_or_create_cell_mut(col)` — needs `.clone()` or rename to `get_or_create_cell`
- [x] 3.2 Check `writer/xlsx.rs` for `sorted_cells()` / `written_cells()` callers:
  - Return type change `Vec<&Cell>` → `Vec<Cell>` — iterator patterns may need minor adjustment (`.iter()` → `.into_iter()` or remove `&`)
- [x] 3.3 Check `writer/styles.rs` for `sorted_cells()` / `cell_count()` callers

- [x] 3.4 Run `cargo test` — all tests pass

## 4. Write new regression tests for the fixed path

- [x] 4.1 In `src/model/worksheet.rs`:
  - `test_row_getcell_via_getrow_persists_value`: `getRow().getCell("A").value = "x"` → re-read from worksheet → value matches
  - `test_row_getcell_via_getrow_persists_style`: `getRow().getCell("A").style = {...}` → re-read → style matches
  - `test_row_getcell_via_getrow_preexisting_cell`: addRow → getRow → getCell → mutate → verify via worksheet
  - `test_row_getcell_via_getrow_orphan_not_created`: verify that a cell NOT created via getCell doesn't leak (phantom cell guard — `written_cells` filter)

- [x] 4.2 Run `cargo test` — confirm no regressions, new tests pass

## 5. Final verification

- [x] 5.1 `cargo test` — full suite green
- [x] 5.2 `cargo clippy` — no new warnings
- [x] 5.3 `cargo fmt` — formatting clean
- [x] 5.4 `pnpm build` — native addon builds
- [x] 5.5 `pnpm test` — JS integration tests pass

## 6. JS integration / E2E tests (vitest)

JS-level tests validate the fix through the napi FFI boundary and through the full write/read pipeline. These go in `__test__/` alongside existing tests.

### 6.1 Pure in-memory (no I/O) — `__test__/worksheet.test.ts`

Fastest validation: prove Rust Arc fix translates correctly across napi boundary without any file I/O.

- [x] 6.1.1 `getRow().getCell('A').value = 'str'` on fresh row (cell doesn't pre-exist):
  - `new Worksheet('Test')`
  - `ws.getRow(1).getCell('A').value = 'Hello!'`
  - `ws.getCell('A1').value.string` → `'Hello!'`
  - `ws.getCell('A1').value.valueType` → `'String'`

- [x] 6.1.2 Same pattern for Number (+ date rounding guard):
  - `ws.getRow(2).getCell('B').value = 42`
  - Read back → number 42

- [x] 6.1.3 Same pattern for Boolean:
  - `ws.getRow(3).getCell('C').value = true`
  - Read back → boolean true

- [x] 6.1.4 Style mutation via same path:
  - `ws.getRow(4).getCell('A').style = { font: { bold: true } }`
  - Read back → font.bold === true

- [x] 6.1.5 Existing cell mutation via row.getCell (pre-existing cell from addRow):
  - `ws.addRow([10])` → `ws.getRow(1).getCell(1).value = 99`
  - `ws.getCell('A1').value.number` → 99
  - (Regression guard for v0.4.0 interior mutability — currently tested, adapt to new section)

### 6.2 Round-trip (write → read with excelrs) — `__test__/workbook_xlsx.test.ts`

Proves the writer iterates the same cell HashMap the mutation went into.

- [x] 6.2.1 `getRow().getCell().value` survives write/read round-trip:
  - `new Workbook()`, `addWorksheet('Test')`
  - `ws.getRow(1).getCell('A').value = 'RoundTrip!'`
  - `await wb.xlsx.write()`, read into fresh workbook
  - `wb2.getWorksheet('Test')!.getCell('A1').value.string` → `'RoundTrip!'`

- [x] 6.2.2 Multiple cells created via row.getCell on different rows:
  - Row 1: `getCell('A')` = 10, `getCell('B')` = 20
  - Row 5: `getCell('A')` = spare row with data
  - Round-trip: both rows present, values intact

- [x] 6.2.3 Value set on row.getCell coexists with addRow values:
  - `addRow(['header'])` (creates row 1, A1 = 'header')
  - `getRow(1).getCell('B').value = 'computed'` (creates B1 on same row)
  - Round-trip: A1 = 'header', B1 = 'computed'

- [x] 6.2.4 Phantom cell prevention (written_cells filter not broken):
  - `ws.getRow(10)` (creates empty row, no cells)
  - Round-trip: row 10 should NOT appear in output (or appear empty — writer filters phantom cells)
  - Read back: `rowCount ≥ 10` is acceptable (rowCount = max row index), but no phantom cell values

### 6.3 Cross-lib E2E (write with excelrs → read with ExcelJS) — `__test__/workbook_xlsx.test.ts`

Proves our .xlsx output is valid for other consumers.

- [x] 6.3.1 Single cell created via `getRow().getCell()`:
  - Write with excelrs, load buffer with `exceljs.Workbook.xlsx.load()`
  - `wbjs.getWorksheet('Test')!.getCell('A1').value` → expected string

- [x] 6.3.2 Cells across multiple rows created via `getRow().getCell()`:
  - Write with excelrs (same setup as 6.2.2)
  - Read with ExcelJS: all cell values present

- [x] 6.3.3 Style applied via `getRow().getCell().style = {...}`:
  - Write with excelrs
  - Read with ExcelJS: style (font.bold) preserved
  
- [x] 6.3.4 Run `pnpm test` — all JS tests pass
