# Proposal: v1.1.0 — Tables

## Why

`excelrs` reached the v1.0.0 drop-in ExcelJS-compat milestone, but the parity
matrix still lists **Tables** as `planned` / `targeted` for v1.1.0. ExcelJS
users expect `ws.addTable` / `ws.getTable(s)` / `ws.removeTable` with a
`Table` / `TableColumn` / `TableRow` model and `xl/tables/tableN.xml` on disk.
Today those calls are missing, so any workbook relying on tables fails or
round-trips them as plain cells — breaking the compat promise for a common,
data-heavy feature.

## What Changes

- Add **Tables** as a first-class worksheet feature with full read/write round-trip:
  - `ws.addTable(opts)` — build a table over a cell range with header, data, and optional totals rows.
  - `ws.getTable(name)` / `ws.getTables()` — retrieve table(s) by name / all.
  - `ws.removeTable(name)` — drop a table (and its `xl/tables/tableN.xml` part + relationship).
  - `TableColumn` (name, totalsRowLabel/Function) and `TableRow` model types.
  - Header row styling, totals row, and `autoFilter` integration so a filtered table survives round-trip.
- Emit/parse `xl/tables/tableN.xml` (`<table>` `<autoFilter>` `<tableColumns>` `<tableStyleInfo>` `<totalsRowCount>`) plus the `table` relationship in the sheet `.rels`.
- **BREAKING**: none — strictly additive. Minor version bump (`1.0.0` → `1.1.0`) signals new capability, no existing API behavior changes.

## Capabilities

### New Capabilities

- `tables`: Worksheet tables — `ws.addTable` / `ws.getTable(s)` / `ws.removeTable`, `Table` / `TableColumn` / `TableRow` model, `xl/tables/tableN.xml` read/write, header+totals rows, and `autoFilter` integration.

### Modified Capabilities

- `exceljs-parity`: v1.1.0 advances the parity matrix `Tables` `planned`/`targeted` → `shipped`. The release-recording requirement gains a v1.1.0 scenario.
<!-- autoFilter element read/write is unchanged; table↔autoFilter interaction is handled inside the tables feature, not as an auto-filter requirement change. -->

## Impact

- **Code**: new model types in `src/model/` (e.g. `table.rs`); new `xl/tables/tableN.xml` reader/writer loop in `src/reader/xlsx.rs` + `src/writer/xlsx.rs`; sheet `.rels` registration (reuses existing rels manager, same plumbing as comments/images/hyperlinks); napi bridge in `src/lib.rs` exposing `ws.addTable` / `ws.getTable(s)` / `ws.removeTable`.
- **APIs**: additive napi surface on `Worksheet`. No changes to existing public types.
- **Dependencies**: none added; reuses `quick-xml`, `zip`, existing rels manager.
- **Specs**: advances `exceljs-parity` matrix `Tables` `planned` → `shipped`; new `tables` capability spec.
- **Parity domain**: closes one remaining "targeted" v1.x parity-matrix row (Tables).
