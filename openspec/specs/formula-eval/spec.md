## Purpose

Add opt-in formula evaluation to excelrs behind the `formula-eval` Cargo
feature, enabling computed values to be cached on cells and emitted as `<v>`
alongside `<f>` in written XLSX output.

## Requirements

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
cells in the worksheet, caching results on each cell. If a formula in a
cell cannot be parsed (syntax error), the evaluator SHALL cache `#VALUE!`
in that cell and continue evaluating remaining cells rather than aborting
the entire recalculation.

#### Scenario: Recalculate evaluates all formula cells

- **WHEN** calling `worksheet.recalculate()` on a sheet with formula cells
- **THEN** every formula cell SHALL have a cached value populated

#### Scenario: Recalculate isolates parse errors per-cell

- **WHEN** a worksheet contains a formula cell with a syntax error (e.g.
  `=1++`) and a valid formula cell `=2+2`, and `recalculate()` is called
- **THEN** the valid cell SHALL be cached with its computed value and the
  parse-error cell SHALL be cached with an error value; no `Err` SHALL
  propagate from `recalculate`

### Requirement: Streaming path does not evaluate

The streaming reader (`WorkbookStream`) SHALL preserve formula strings only.
`Cell::cachedValue` SHALL return `null` for cells from the streaming path.

#### Scenario: Streaming cell has no cached value

- **WHEN** a formula cell is read via `WorkbookStream`
- **THEN** `cachedValue` SHALL be `null`

### Requirement: Date cells participate in arithmetic as serial numbers

Excel stores dates as serial numbers (f64 days since 1900-01-01). The
evaluator SHALL treat `Value::Date` as its underlying serial number in all
numeric contexts — arithmetic, range aggregation, and type coercion.

#### Scenario: Date cell in arithmetic

- **WHEN** cell A1 holds a date and the formula `=A1+1` is evaluated
- **THEN** the result SHALL be the next serial number (next day)

#### Scenario: Date cells in SUM range

- **WHEN** evaluating `=SUM(A1:A3)` where A1, A2, A3 are date cells
- **THEN** the result SHALL be the sum of their serial numbers

#### Scenario: Date cell in MIN/MAX range

- **WHEN** evaluating `=MIN(A1:A3)` where A1, A2, A3 are date cells
- **THEN** the result SHALL be the earliest date (smallest serial number)

### Requirement: MIN and MAX with zero numeric arguments return 0

When `MIN()` or `MAX()` is called with no numeric arguments, the result
SHALL be `0` (matching Excel behavior). Only `AVERAGE` returns `#DIV/0!`
when given no numeric arguments.

#### Scenario: Empty MIN returns zero

- **WHEN** evaluating `=MIN()` with zero numeric arguments
- **THEN** the result SHALL be `0`

#### Scenario: Empty MAX returns zero

- **WHEN** evaluating `=MAX()` with zero numeric arguments
- **THEN** the result SHALL be `0`

### Requirement: Parse errors are isolated per-cell during recalculation

When `recalculate()` encounters a formula that cannot be parsed, the
evaluator SHALL cache a cell-level error (`#VALUE!`) on that cell and
continue evaluating remaining formula cells. A single unparseable formula
SHALL NOT prevent other cells from receiving cached values.

#### Scenario: Parse error in one cell does not block others

- **WHEN** cell A1 contains `=1+A1` (parse error) and cell B1 contains `=2+2`
  and `recalculate()` is called
- **THEN** B1 SHALL receive cached value `4` and A1 SHALL receive a cached
  error value

### Requirement: Workbook.recalculate recalculates all worksheets with cross-sheet context

The JS-visible `Workbook` class SHALL expose `recalculate()`. It SHALL iterate
every worksheet and recalculate each formula cell with full workbook context,
so cross-sheet references (e.g. `Sheet2!A1`) resolve to live values.

#### Scenario: Cross-sheet reference resolves during workbook recalc

- **WHEN** `Sheet1!B1` contains `=Sheet2!A1`, `Sheet2!A1` contains `42`, and
  `workbook.recalculate()` is called
- **THEN** `Sheet1!B1` cached value SHALL be `42`

#### Scenario: All formula cells across sheets receive cached values

- **WHEN** a workbook has formula cells on multiple worksheets and
  `recalculate()` is called
- **THEN** every formula cell in every worksheet SHALL have its cached value
  populated
- **AND** parse / circular-reference errors SHALL be isolated per cell (no abort)

### Requirement: Worksheet.recalculate is exposed on the JS class

The JS-visible `Worksheet` class SHALL expose `recalculate()` (currently
internal-only). It SHALL recalculate formula cells within that worksheet and
cache computed scalars. Cross-sheet references to OTHER sheets resolve to
`#REF!` because no workbook context is available at the worksheet scope.

#### Scenario: Worksheet recalc populates cached values

- **WHEN** `worksheet.recalculate()` is called on a sheet with `=1+2` in a cell
- **THEN** that cell's cached value SHALL be `3`

#### Scenario: Cross-sheet ref from a worksheet-only recalc is unresolvable

- **WHEN** `worksheet.recalculate()` is called and a formula references another
  sheet
- **THEN** the referenced cell SHALL cache `#REF!`
- **AND** the recalculation SHALL still complete (per-cell isolation)
