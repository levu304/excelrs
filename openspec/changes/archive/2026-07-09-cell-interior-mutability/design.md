## Context

`excelrs` is a Rust/napi-rs native addon mirroring the exceljs API. napi-rs v3 only ever returns **owned** types across the FFI boundary — `#[napi]` functions cannot return `&Cell`/`&Row`. Today `Row` and `Column` already solve this with `Arc<Mutex<>>` (`worksheet.rs:11-12`): any clone of a `Worksheet` shares the same row map and column vector, so `ws.addRow(...)` / `ws.setColumns(...)` mutate the workbook's internal model.

`Cell` is the lone holdout. It is `#[derive(Clone)]` as a plain value type (`cell.rs:105`). `Worksheet::get_cell_by_rc` → `Row::get_cell_by_col_num` clones the cell out of the map (`row.rs:69-77`), so the JS `Cell` is a copy. Setting `cell.style = {...}` or `cell.value = x` mutates only the copy; the worksheet never sees it. The spec §6.2 (line 584) and the README Limitations both document this as a known v0.2-deferred limitation and name `Arc<Mutex<>>`/`RefCell` as the intended fix.

## Goals / Non-Goals

**Goals:**
- `ws.getCell('A1').style = {...}` and `ws.getCell('A1').value = x` persist into the owning worksheet (exceljs-compatible chainable mutation).
- Keep the public napi getter/setter JSON surface identical — no JS-visible signature change.
- Reuse the existing `Arc<Mutex<>>` interior-mutability pattern already proven in `Row`/`Column`.

**Non-Goals:**
- No merged cells, streaming, formula evaluation, or CSV/XLS/XLSB reading.
- No new public methods (no new `ws.setCell(...)`).
- No change to column-default style semantics or to the existing `setCellStyle` path.

## Decisions

### D1. Wrap `Cell` data in `Arc<Mutex<CellInner>>`

Split `Cell` into a public `#[napi]` handle holding `Arc<Mutex<CellInner>>` and a private `CellInner` struct carrying `address`, `row`, `col`, `value`, `formula`, `style`. `Clone` is derived cheaply (clones the `Arc`). Every getter/setter locks the mutex, reads/writes `CellInner`, and returns owned copies (as napi requires).

- **Why this over `RefCell`**: `Row`/`Column` already use `Mutex` and the worksheet is accessed from async/Tokio contexts; keeping one consistent interior-mutability choice avoids mixing `RefCell` (not `Sync`, would break sharing the handle across await points) with `Mutex`. Reusing `Arc<Mutex<>>` also matches the established repo pattern exactly.
- **Alternatives considered**: (a) Return `&Cell` — impossible under napi-rs v3. (b) Add a `ws.setCell(address, cell)` writeback method — new public API, diverges from exceljs, doesn't fix `cell.value = x` ergonomics. (c) Keep `setCellStyle` only — leaves the documented trap unfixed, violates P1.

### D2. Accessors return shared handles, not clones of data

`Row::get_cell_by_col_num` / `get_cell_by_col_letter` and `Worksheet::get_cell_by_rc` / `get_cell_by_address` return a `Cell` whose `Arc` points at the cell already stored in the row's `HashMap<u32, Cell>`. Because `Cell` now contains an `Arc`, storing it in the map and returning it both share the same `CellInner`. Cells created on demand for empty addresses still get inserted into the row map (via `get_or_create_cell_mut`) so the handle is backed by real shared state.

### D3. Preserve `setCellStyle` and reader paths

`Worksheet::set_cell_style` and the reader's `insert_cell_style`/`insert_cell_value` mutate the locked row map and call `get_or_create_cell_mut(col)` then `set_style_raw`/`set_value_raw` on the in-map `Cell` (which itself locks its `Arc`). These keep working unchanged; they are already correct because they go through the map, not a clone.

## Risks / Trade-offs

- **[Mutex poisoning]** → A panic while holding the cell lock would poison it. Mitigate: lock with `.lock().expect("Cell lock poisoned")` consistent with the existing `rows`/`columns` locks (`worksheet.rs`). Cell accessors are simple and non-panicking.
- **[Non-`Copy` `Cell`]** → `Cell` can no longer be `Copy`; internal code that relied on implicit copies must clone the `Arc` (cheap). The existing `.clone()` calls in `get_row`/`add_row` already do this, so impact is localized.
- **[Lock-ordering]** → Cell locks nest inside the row-map lock (cell lock taken while holding the rows lock). This is unidirectional (never the reverse) so no deadlock. Keep all cell locking done *inside* an already-held rows lock where possible; standalone `Cell` getters (called from JS without the rows lock) lock only the cell — safe.
- **[Perf]** → Per-call mutex lock on every `value`/`style` read/write. Negligible vs. spreadsheet-scale work and identical to the cost already paid for row/column access. No benchmark regression expected.

## Migration Plan

- No behavioral migration for callers — this is a correctness fix to existing methods.
- Deploy with a normal semver minor bump (v0.3.0 → v0.4.0). No rollback needed beyond a standard revert; the change is contained to `src/model/{cell,row,worksheet}.rs`.
- Update `docs/spec.md` §6.2 mutation-semantics note and the README Limitations section to remove the "use `setCellStyle`" caveat (follow-up doc task, not blocking).

## Open Questions

- None blocking. Whether to also expose cell `value` mutation persistence for `getCell().value = x` is in scope (same root cause, same fix) — included, not deferred.
