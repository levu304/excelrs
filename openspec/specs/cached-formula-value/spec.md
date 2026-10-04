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

### Requirement: Writer emits `<f>` and `<v>` for each cached scalar on Formula cells

When a `Formula` `CellValue` carries a cached scalar, the writer SHALL emit
`<f>{formula}</f><v>{cached}</v>`.

#### Scenario: Formula cell with a cached scalar emits both elements

- **WHEN** the writer serializes a `Formula` `CellValue` whose cached field is populated
- **THEN** it SHALL emit `<f>{formula}</f>` followed by `<v>{cached}</v>` for that cell

#### Scenario: Formula cell without a cached scalar emits no `<v>`

- **WHEN** the writer serializes a `Formula` `CellValue` whose cached field is absent
- **THEN** it SHALL emit `<f>{formula}</f>` and no `<v>` element for that cell

### Requirement: Writer sets the Formula cell `t` attribute by cached-scalar priority

The `t` attribute emitted for the `Formula` arm SHALL check cached scalar fields in the
priority order `number → string → boolean → error_value → date_serial`, matching the order
the value-writing arm uses. `number` and `date_serial` SHALL produce no `t` attribute.
Precedence SHALL prevent a `t` attribute that contradicts the emitted `<v>` content.

#### Scenario: Boolean cached value wins over a simultaneously present string

- **WHEN** a `Formula` `CellValue` carries both a `boolean` and a `string` cached field
- **THEN** the emitted `t` SHALL be `"b"` and the `<v>` SHALL contain the boolean's value

#### Scenario: Numeric cached value emits no type attribute

- **WHEN** a `Formula` `CellValue` carries a `number` cached field
- **THEN** the cell SHALL be emitted with no `t` attribute and `<v>` containing the number

### Requirement: Writer emits a `date_serial` cached branch on Formula cells

The `Formula` arm of the writer SHALL emit a `date_serial` cached branch, mirroring the
`Date` arm, so a cached date scalar round-trips through the `<v>` element.

#### Scenario: Cached date serial on a Formula cell

- **WHEN** the writer serializes a `Formula` `CellValue` whose cached scalar is a `date_serial`
- **THEN** it SHALL emit the date serial as the `<v>` content with no `t` attribute

### Requirement: Cell.cachedValue exposes only recalculated cached scalars

The `Cell.cachedValue` getter SHALL return a cached scalar only when the cell was evaluated
via `Worksheet::recalculate()`. For every other state the getter SHALL return `null`.

#### Scenario: Recalculated cell exposes its cached value

- **WHEN** `Worksheet::recalculate()` evaluates a `Formula`-typed cell
- **THEN** `Cell.cachedValue` SHALL return the evaluated cached scalar

#### Scenario: Authored cell does not expose a cached value

- **WHEN** a `Formula`-typed cell has a cached scalar set through the reader or the JS value setter but has not been recalculated
- **THEN** `Cell.cachedValue` SHALL return `null`

### Requirement: Cached values from authoring and reader paths are not exposed as recalc results

A cached scalar set through the reader's raw cell-value and cell-formula insertion paths, or
through the JS value setter, SHALL NOT be observable through `Cell.cachedValue`. Only the
recalculation path SHALL make a cached scalar observable there.

#### Scenario: Reader-inserted cached scalar is hidden

- **WHEN** the reader inserts a `Formula` cell carrying a cached scalar
- **THEN** `Cell.cachedValue` SHALL return `null` until recalculation runs

#### Scenario: JS setter cached scalar is hidden

- **WHEN** a JS caller sets a `Formula` cell's value carrying a cached scalar
- **THEN** `Cell.cachedValue` SHALL return `null` until recalculation runs

### Requirement: Excel-authored cached formula reads back

A committed fixture containing `<f>..</f><v>..</v>` (authored by Excel or ExcelJS via
`result`) SHALL read back so the cached value is available.

#### Scenario: disk/Excel-authored cached formula returns cached scalar

WHEN a workbook authored in Excel (or by ExcelJS with `{ formula, result }`) containing
`<f>A2+B2</f><v>3</v>` is read
THEN `cell.value` is `3` and `cell.formula` is `"A2+B2"`.

### Requirement: date cached formula round-trip test coverage

The JS-authored date-formula round-trip scenario SHALL be covered by an automated test in
`__test__/cached-formula.test.ts`.

#### Scenario: JS-authored cached date formula round-trips as bare number

WHEN a cell is assigned `{ formula: "DATE(2025,1,1)", dateSerial: 45657 }` and round-tripped
THEN `cell.value` is `45657` (a bare number, not a JS `Date`).

### Requirement: Legacy array formulas degrade to plain formulas preserving text and cached value

A cell carrying `<f t="array" ref="...">` SHALL read as a `Formula` cell whose formula text and cached `<v>` scalar are preserved, and SHALL write back as a plain `<f>` with its cached `<v>`. The array type and ref range are not preserved. Recalculation-dependent array (spill) semantics stay out of scope.

#### Scenario: Array cell reads as a plain formula with its cached value

- **WHEN** a workbook whose `C1:C3` each carry `<f t="array" ref="C1:C3">SUM(A1:A3*1)</f><v>6</v>` is read
- **THEN** each of `C1`, `C2`, `C3` SHALL report `formula` `"SUM(A1:A3*1)"` and `value` `6`

#### Scenario: Array cell writes back as a plain formula

- **WHEN** such a workbook is written back to XLSX
- **THEN** each cell SHALL emit `<f>SUM(A1:A3*1)</f><v>6</v>` with no `t` or `ref` attribute on the `<f>`

#### Scenario: Round-tripped array file opens without repair

- **WHEN** the written workbook is opened in Excel or read by ExcelJS
- **THEN** it SHALL open without a repair prompt, presenting three independent plain formulas
