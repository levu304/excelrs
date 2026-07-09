## 1. Cell interior mutability

- [x] 1.1 In `src/model/cell.rs`, extract mutable fields (`address`, `row`, `col`, `value`, `formula`, `style`) into a private `struct CellInner` and make public `Cell` hold `Arc<Mutex<CellInner>>`.
- [x] 1.2 Reimplement `Cell` getters (`value`, `address`, `row`, `col`, `formula`, `style`) to lock the mutex and return owned copies; reimplement `set_value`/`set_style` setters to lock and write into `CellInner`.
- [x] 1.3 Move `set_value_raw`/`set_style_raw`/`set_formula` to operate on the locked inner (called by reader/row paths) and keep `compute_address` as an associated fn.
- [x] 1.4 Preserve `Clone` on `Cell` (cheap `Arc` clone) so `.clone()` calls in `get_row`/`add_row` and the row `HashMap` keep working; `Cell` is no longer `Copy`.

## 2. Row / Worksheet accessors return shared handles

- [x] 2.1 In `src/model/row.rs`, change `get_cell_by_col_num` / `get_cell_by_col_letter` to return the cell stored in `self.cells` (which now carries a shared `Arc`) rather than a fresh clone; for absent cells, insert via `get_or_create_cell_mut` then return.
- [x] 2.2 In `src/model/worksheet.rs`, confirm `get_cell_by_rc` / `get_cell_by_address` return the shared-handle cell through `Row::get_cell_by_col_num` (no extra clone of data).
- [x] 2.3 Verify `get_or_create_cell_mut` (row.rs) and `set_cell_style`/`insert_cell_*` (worksheet.rs) still operate on the in-map `Cell` and thus mutate the shared `Arc` correctly.

## 3. Verification

- [x] 3.1 Run `cargo test` — all existing Rust unit tests pass (cell/row/worksheet modules).
- [x] 3.2 Run `cargo clippy -- -D warnings` and `cargo fmt -- --check` — clean.
- [x] 3.3 Add a Rust regression test asserting `ws.getCell('A1').style = {...}` and `.value = x` persist through `Worksheet` clone (simulating FFI clone) and survive a write/read round-trip.
- [x] 3.4 Run `pnpm test` — JS integration tests pass; add a JS test that sets `cell.style`/`cell.value` on a fetched cell, writes, reads back, and asserts persistence.

## 4. Docs (follow-up, non-blocking)

- [x] 4.1 Update `docs/spec.md` §6.2 mutation-semantics note to state cell-level mutation now persists.
- [x] 4.2 Remove the "use `ws.setCellStyle()` for reliable cell-level style setting" caveat from README Limitations.
