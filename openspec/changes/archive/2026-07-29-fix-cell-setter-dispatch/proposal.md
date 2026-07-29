## Why

PR #45 added object-shaped dispatch to the `Cell.value` setter (incl. `formula`,
`richText`, `hyperlink`, `valueType`). Review confirmed a **critical regression**:
setting a formula via the object setter — `cell.value = { formula: "SUM(A1:B1)" }`
— is silently dropped on XLSX write because the writer reads `CellInner.formula`,
while the object setter only populates `CellValue.formula`. The same dispatch also
carries dead parsing and accepts arbitrary `valueType` strings that produce empty
cells. This change fixes the formula bug and hardens the dispatch.

## What Changes

- **Fix formula persistence**: formulas set via the object setter now survive
  XLSX write (populate `CellInner.formula`, or make the writer read the formula
  from `CellValue`). Without this, `cell.value = { formula: "..." }` emits a
  cell with no `<f>` element.
- **Remove dead parsing** in the `valueType` arm of the setter: `formula`,
  `hyperlink`, `hyperlink_text`, and `rich_text` reads there are unreachable
  (the `richText`/`hyperlink`/`formula` key branches already consume those keys).
- **Validate `valueType`** before accepting it: reject unknown discriminants
  (e.g. `"Banana"`) instead of emitting a silently empty cell, OR document the
  fallback. Decision captured in design.md.
- **Document the silent-`Null` fallback** for objects with no recognized key
  (e.g. `{ number: 5 }` without `valueType`), so callers understand dropped data.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities

- `cell-value-dispatch`: setter requirement changes — formulas set via the object
  setter must round-trip to XLSX; `valueType` must be validated against the known
  discriminant set; dead/unreachable dispatch branches removed.

## Impact

- `src/model/cell.rs` — `set_value` setter (object dispatch arm).
- `src/writer/xlsx.rs` — `write_cell_xml` formula emission (reads `CellInner.formula`).
- `__test__/cell.test.ts`, `__test__/rich-text.test.ts` — add formula round-trip
  and `valueType` validation tests.
- No public API signature change; behavior change only (formulas now persist,
  unknown `valueType` now errors instead of silently emptying the cell).
