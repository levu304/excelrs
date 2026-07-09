## Why

`excelrs` advertises drop-in exceljs compatibility (design principle P1), but `ws.getCell('A1').style = {...}` and `ws.getCell('A1').value = x` silently lose data today. Because `#[napi]` functions return owned clones across the FFI boundary, the `Cell` returned by `getCell` is a copy — mutations never reach the worksheet's internal model. The README and spec §6.2 both document this as a known v0.2-deferred limitation and explicitly name `Arc<Mutex<>>` / `RefCell` as the intended fix. Until it ships, every exceljs port that does `cell.style = {...}` on a fetched cell is a correctness trap. Fixing it closes the only deferred item that actively violates our headline compatibility promise.

## What Changes

- Make `Cell` share mutable state across clones via interior mutability (`Arc<Mutex<CellInner>>`), matching the pattern already used by `Row` and `Column` (`worksheet.rs:11-12`).
- `Worksheet.getCell*` (and `Row.getCell*`) will now return a handle whose `value`/`style` mutations persist into the worksheet.
- Public napi getter/setter JSON surface stays byte-for-byte identical — invisible to JS callers.
- `ws.setCellStyle(row, col, style)` and the reader's `insert_cell_style` / `insert_cell_value` paths continue to work (they mutate through the locked row map today).
- Removes the README caveat "use `ws.setCellStyle()` for reliable cell-level style setting".

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `cell-mutation`: Requirement change — cell-level `value` and `style` setters on a `Cell` obtained via `getCell`/`getRow().getCell` must persist into the owning worksheet (exceljs-compatible chainable mutation). Previously this was documented as unsupported / clone-on-read lossy. No new FFI signatures; this is a behavioral contract change on existing methods.

## Impact

- **Code**: `src/model/cell.rs` (wrap fields in `CellInner` + `Arc<Mutex<>>`), `src/model/row.rs` (`get_cell_by_col_num` / `get_cell_by_col_letter` return shared-handle cells; `get_or_create_cell_mut` stays internal), `src/model/worksheet.rs` (`get_cell_by_rc` / `get_cell_by_address` return shared-handle cells). `Cell` becomes non-`Copy`; internal signatures taking `&mut Cell` switch to locking.
- **API**: No JS-facing signature changes. This is purely a correctness fix to existing behavior.
- **Dependencies**: None added. Uses `std::sync::{Arc, Mutex}` already in the crate.
- **Tests**: Extend `cargo test` + `pnpm test`. Add a regression test that sets `style`/`value` on a fetched cell, writes, reads back, and asserts persistence (round-trip + drop-in compat).
- **Spec**: `docs/spec.md` §6.2 mutation-semantics note and README Limitations section must be updated to reflect the fix (tracked as follow-up after implementation).

## Non-Goals

- No merged-cell support, streaming write, formula evaluation, or CSV/XLS/XLSB reading.
- No new public methods (e.g. no new `ws.setCell(...)`). `setCellStyle` remains supported.
- No change to column-level default style semantics.
