## Why

excelrs advertises drop-in exceljs compatibility (design principle P1). The
roadmap (docs/spec.md §9.3) lists "CSV read/write" as a future capability;
nothing ships it. CSV is the most common tabular interchange format, yet
today there is no way to import or export it from exceljs — users must
round-trip through another tool. This change closes that roadmap item as a
single, focused release (matching the single-feature shape of v0.7.0 and
v0.8.0).

## What Changes

- Add a `WorkbookCsv` async handle, obtained via the `wb.csv` getter
  (mirrors `wb.xlsx`), sharing the same `Arc<Mutex<WorkbookInner>>`.
- `csv.read(buf)` / `csv.readFile(path)`: parse RFC 4180 CSV into a single
  Worksheet (named "Sheet1"). Light numeric inference for round-trip
  fidelity; optional `delimiter`.
- `csv.write()` / `csv.writeFile(path)`: serialize the **first** worksheet
  (CSV is single-sheet) to RFC 4180 CSV. Formula cells emit their cached
  value (no evaluation). Optional `delimiter` + `withBom`.
- Types: `WorkbookCsv` class + options in `index.d.ts`.

## Capabilities

### New Capabilities

- `csv`: read/write RFC 4180 CSV via a `Workbook.csv` async handle.
  Single-worksheet; numeric inference on read; formula-cached-value on
  write. exceljs-compatible surface (`Workbook.csv.readFile`/`writeFile`).

### Modified Capabilities

*(none — no existing specs are modified)*

## Impact

- **Code**: new `src/csv.rs` (parse + serialize); `WorkbookCsv` handle
  mirroring `WorkbookXlsx`; `csv` getter on `Workbook`.
- **API**: new `WorkbookCsv` interface + `csv` getter in `index.d.ts`.
  No breaking changes.
- **Dependencies**: none added — a manual RFC 4180 parser/serializer
  (~30 lines) covers the well-bounded quoting rules.
- **Tests**: Rust unit tests for parse/serialize round-trip; JS vitest
  csv round-trip + numeric inference + exceljs cross-check.
