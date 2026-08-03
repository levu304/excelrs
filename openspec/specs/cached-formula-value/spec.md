# cached-formula-value Specification

## Purpose
Define the behavior for authoring and round-tripping **cached formula results** (the `<v>` paired
with a `<f>`) without depending on a formula-evaluation engine. A formula cell authored with an
explicit cached scalar SHALL be serialized as `<f>{formula}</f><v>{cached}</v>` and SHALL read
back so that `cell.value` returns the cached scalar and `cell.formula` returns the formula text.

This is the minimal, engine-independent half of issue #54 ("Formula cached results cannot be
authored on write"). It does **not** attempt in-process formula evaluation — cached values are
supplied by the caller (JS authoring) or by Excel itself.
## Requirements
### Requirement: setter accepts cached scalar fields on a Formula cell

`Cell.value = { valueType: "Formula" | formula, <cached scalar field> }` SHALL store the supplied
cached scalar on the `Formula` `CellValue` alongside the formula string, folding into the
existing `number`, `string`, `boolean`, `error_value`, `date_serial` fields (no new fields, no
new variants).

#### Scenario: numeric cached value round-trips

WHEN a cell is assigned `{ valueType: "Formula", formula: "SUM(A2:B2)", number: 3 }`, written
to xlsx, and read back
THEN `cell.value` is `3` (a number), and `cell.formula` contains the formula text.

#### Scenario: boolean cached value round-trips

WHEN a cell is assigned `{ formula: "A1>B1", boolean: true }` and round-tripped
THEN `cell.value` is `true` and `cell.formula` is `"A1>B1"`.

#### Scenario: string cached value round-trips

WHEN a cell is assigned `{ formula: "CONCAT(\"a\",\"b\")", string: "ab" }` and round-tripped
THEN `cell.value` is `"ab"` and `cell.formula` is present.

#### Scenario: error cached value round-trips

WHEN a cell is assigned `{ formula: "1/0", errorValue: "#DIV/0!" }` and round-tripped
THEN `cell.value` is the error and `cell.formula` is `"1/0"`.

#### Scenario: date cached value round-trips

WHEN a cell is assigned a formula with a `dateSerial` cached value and round-tripped
THEN the date-serial scalar is preserved and the formula text is preserved.

#### Scenario: formula authored without a cached value still reads back

WHEN a cell is assigned `{ formula: "SUM(A1:B1)" }` with no cached scalar and round-tripped
THEN `cell.value` is `null` and `cell.formula` is `"SUM(A1:B1)"` (no regression vs. current
behavior).

### Requirement: writer emits `<v>` for each cached scalar on Formula cells

When a `Formula` `CellValue` carries a cached scalar, the writer `"Formula"` arm SHALL emit
`<f>{formula}</f><v>{cached}</v>`. The arm already emits `number`/`string`/`boolean`/
`error_value`; this change adds the `date_serial` branch (mirrors the `Date` arm).

#### Scenario: date cached value persists `<v>`

WHEN a formula cell is assigned `{ formula: "DATE(2025,1,1)", dateSerial: 45657 }` and round-tripped
THEN the written xlsx contains `<f>DATE(2025,1,1)</f><v>45657</v>` and reads back
`cell.value` is `45657`.

### Requirement: Excel-authored cached formula reads back

A committed fixture containing `<f>..</f><v>..</v>` (authored by Excel or ExcelJS via
`result`) SHALL read back so the cached value is available.

#### Scenario: disk/Excel-authored cached formula returns cached scalar

WHEN a workbook authored in Excel (or by ExcelJS with `{ formula, result }`) containing
`<f>A2+B2</f><v>3</v>` is read
THEN `cell.value` is `3` and `cell.formula` is `"A2+B2"`.

