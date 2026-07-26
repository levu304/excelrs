# Spec: Row-Cell Integrity (Clone Safety)

## Status

**Delta spec** — bug fix, no API surface change.

## Affected Types

- `Row` — internal only (`src/model/row.rs`)
  - Field: `cells: HashMap<u32, Cell>` → `cells: Arc<Mutex<HashMap<u32, Cell>>>`
  - No change to `#[napi]` public API (getters, setters, methods)
  - No change to TypeScript declarations

## Behavioral Contract (unchanged, now enforced)

- `worksheet.getRow(n).getCell(col).value = x` MUST persist the value into the worksheet for write
- `worksheet.getRow(n).getCell(col).style = s` MUST persist the style
- This applies regardless of whether the cell pre-existed or was created by `getCell`
- This applies regardless of whether the row pre-existed or was created by `getRow`

## Non-Affected

- `worksheet.getCell()` — already correct, no changes
- `Cell` — no changes (already uses `Arc<Mutex<CellInner>>`)
- `Worksheet` — no changes
- Any test that currently passes — must continue passing
- Writer behavior — no change (reads from same cell HashMap)
