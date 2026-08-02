## ADDED Requirements

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

## MODIFIED Requirements

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
