## MODIFIED Requirements

### Requirement: writer emits `<v>` for each cached scalar on Formula cells

When a `Formula` `CellValue` carries a cached scalar, the writer `"Formula"` arm SHALL emit
`<f>{formula}</f><v>{cached}</v>`. The arm already emits `number`/`string`/`boolean`/`error_value`
and now emits `date_serial` (mirrors the `Date` arm).

The `t` attribute emitted via `cell_type_attr` for the `Formula` arm SHALL check cached scalar
fields in the same priority order as the value-writing arm: `number → string → boolean →
error_value → date_serial`. This prevents a mismatch where `t="b"` is emitted alongside a `<v>`
containing a string value (when both `boolean` and `string` are present). `number` and
`date_serial` produce no `t` attribute (fall through to `None`).

#### Scenario: date cached value persists `<v>`

WHEN a formula cell is assigned `{ formula: "DATE(2025,1,1)", dateSerial: 45657 }` and round-tripped
THEN the written xlsx contains `<f>DATE(2025,1,1)</f><v>45657</v>` and reads back
`cell.value` is `45657`.

#### Scenario: t attribute matches v content for boolean+string formula cell

WHEN a formula cell carries both `boolean` and `string` cached scalars (edge case)
THEN the emitted `t` attribute matches the field type whose value is emitted in `<v>` (string takes
priority, matching the value-writing arm).

## ADDED Requirements

### Requirement: Cell.cachedValue is recalc-only

The `Cell.cachedValue` getter SHALL return `null` for any `Formula`-typed cell whose cached scalar
was set by the reader (`set_value_raw` / `insert_cell_value` / `insert_cell_formula`) or by the JS
setter (`set_value`). It SHALL return the cached scalar ONLY when the cell was evaluated via
`Worksheet::recalculate()` (which calls `set_cached_value_raw`).

The implementation enforces this via a per-cell `recalc_only: bool` flag on `CellInner`:

- `recalc_only = false` after `Cell::new`, `set_value_raw`, or `set_value` (authoring/reader paths).
- `recalc_only = true` only after `set_cached_value_raw` (recalc path).
- `cached_value()` returns `None` when `recalc_only` is `false`, even if cached scalar fields are present.

#### Scenario: Excel-authored cached formula cell.cachedValue is null

WHEN a Formula cell read from disk carries `<f>..</f><v>..</v>` (Excel/ExcelJS authored)
THEN `cell.cachedValue` is `null` (reader path sets `recalc_only = false`).

#### Scenario: JS-authored cached formula cell.cachedValue is null

WHEN a cell is assigned `{ formula: "SUM(A2:B2)", number: 3 }` via the JS setter
THEN `cell.cachedValue` is `null` (setter path sets `recalc_only = false`).

#### Scenario: recalc'd formula cell.cachedValue returns scalar

WHEN a Formula cell is evaluated via `Worksheet::recalculate()`
THEN `cell.cachedValue` returns the computed scalar (recalc path sets `recalc_only = true`).

### Requirement: date cached formula round-trip test coverage

The JS-authored date-formula round-trip scenario SHALL be covered by an automated test.

#### Scenario: JS-authored cached date formula round-trips as bare number

WHEN a cell is assigned `{ formula: "DATE(2025,1,1)", dateSerial: 45657 }` and round-tripped
THEN `cell.value` is `45657` (a bare number, not a JS `Date`).
