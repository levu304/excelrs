## Purpose

Add opt-in formula evaluation to excelrs behind the `formula-eval` Cargo
feature, enabling computed values to be cached on cells and emitted as `<v>`
alongside `<f>` in written XLSX output.

## ADDED Requirements

### Requirement: Formula evaluation is opt-in via Cargo feature

The system SHALL gate the formula evaluation code behind a Cargo feature
named `formula-eval`. When the feature is disabled (default), the evaluation
API SHALL be absent from the public surface and no formula-eval dependencies
 SHALL be present on the build graph.

#### Scenario: Evaluation API absent without feature

- **WHEN** the crate is built without `--features formula-eval`
- **THEN** `FormulaEvaluator`, `Cell::cachedValue`, and `Worksheet::recalculate`
  SHALL NOT appear in the public API

### Requirement: Formula evaluator resolves cell and range references

The evaluator SHALL resolve cell references (e.g. `A1`, `'Sheet 2'!B2`) and
range references (e.g. `A1:B3`, `A:A` for whole-column) through the excelrs
data model. Cell references SHALL return the cell's cached scalar value.
Range references SHALL return a 2D array of scalar values for use in array
formulas and aggregate functions.

#### Scenario: Cell reference resolves to live cell value

- **WHEN** evaluating `=A1` on a worksheet where cell A1 contains `42`
- **THEN** the result SHALL be `42`

#### Scenario: Range reference resolves to 2D array

- **WHEN** evaluating `=SUM(A1:B2)` where A1=1, B1=2, A2=3, B2=4
- **THEN** the result SHALL be `10`

#### Scenario: Whole-column reference resolves to used rows

- **WHEN** evaluating `=A:A` where column A has values in rows 1-3 only
- **THEN** the array SHALL contain all used rows in column A, trailing
  rows SHALL be empty

### Requirement: Arithmetic operators respect Excel precedence and coerce types

The evaluator SHALL support `+`, `-`, `*`, `/`, `^`, `%` operators and
comparison operators `=`, `<>`, `<`, `>`, `<=`, `>=`. Arithmetic SHALL coerce
text-to-number where possible. Division by zero SHALL return `#DIV/0!`.
Errors SHALL propagate through nested expressions (sticky errors).

#### Scenario: Operator precedence

- **WHEN** evaluating `=1+2*3`
- **THEN** the result SHALL be `7`

#### Scenario: Division by zero

- **WHEN** evaluating `=1/0`
- **THEN** the result SHALL be `#DIV/0!`

#### Scenario: Error propagation

- **WHEN** evaluating `=1/0+5`
- **THEN** the result SHALL be `#DIV/0!` (error short-circuits)

### Requirement: Built-in function set

The evaluator SHALL implement: SUM, AVERAGE, MIN, MAX, COUNT, COUNTA, IF,
AND, OR, NOT, ABS, ROUND, CONCAT, LEFT, RIGHT, MID, LEN, IFERROR.
Unsupported functions SHALL return `#NAME?`.

#### Scenario: SUM over mixed cell range

- **WHEN** evaluating `=SUM(A1:B2)` where A1=1, B1=2, A2=3, B2=4
- **THEN** the result SHALL be `10`

#### Scenario: IF with boolean condition

- **WHEN** evaluating `=IF(A1>0,"yes","no")` where A1=1
- **THEN** the result SHALL be `"yes"`

#### Scenario: AVERAGE with non-numeric cells ignored

- **WHEN** evaluating `=AVERAGE(A1:A3)` where A1=10, A2="text", A3=20
- **THEN** the result SHALL be `15` (only numbers counted)

### Requirement: Circular reference detection

The evaluator SHALL detect circular references and return `#REF!` rather than
looping infinitely.

#### Scenario: Direct self-reference

- **WHEN** evaluating `=A1` where cell A1 contains `=A1`
- **THEN** the result SHALL be `#REF!`

### Requirement: Cached value storage

The evaluator SHALL store computed scalars on the `CellValue` cached fields
(`number`, `string`, `boolean`, `error_value`) so the XLSX writer emits
`<f>formula</f><v>computed</v>` for evaluated cells.

#### Scenario: Evaluated cell writes cached value

- **WHEN** a cell with formula `=1+2` is evaluated and then written to XLSX
- **THEN** the output SHALL contain `<f>1+2</f><v>3</v>`

### Requirement: Cell.cachedValue getter (JS)

The system SHALL expose a read-only napi getter `Cell.cachedValue` that
returns the evaluated scalar (number, string, boolean, or error string) or
`null` when the cell was not evaluated.

#### Scenario: cachedValue returns computed number

- **WHEN** `cell.formula` is `=1+2` and `cachedValue` is read after evaluation
- **THEN** `cachedValue` SHALL be `3`

#### Scenario: cachedValue returns null for unevaluated formula

- **WHEN** a formula cell has not been evaluated
- **THEN** `cachedValue` SHALL be `null`

### Requirement: Worksheet.recalculate method (Rust)

The system SHALL provide `Worksheet::recalculate` that evaluates all formula
cells in the worksheet, caching results on each cell.

#### Scenario: Recalculate evaluates all formula cells

- **WHEN** calling `worksheet.recalculate()` on a sheet with formula cells
- **THEN** every formula cell SHALL have a cached value populated

### Requirement: Streaming path does not evaluate

The streaming reader (`WorkbookStream`) SHALL preserve formula strings only.
`Cell::cachedValue` SHALL return `null` for cells from the streaming path.

#### Scenario: Streaming cell has no cached value

- **WHEN** a formula cell is read via `WorkbookStream`
- **THEN** `cachedValue` SHALL be `null`
