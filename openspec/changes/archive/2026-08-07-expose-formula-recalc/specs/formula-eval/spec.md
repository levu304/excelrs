## ADDED Requirements

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

> Note: the existing requirements "Cached value storage" and "Cell.cachedValue
> getter (JS)" already cover caching and writer emission. This change only makes
> the recalc trigger reachable from JS and supplies workbook context for
> cross-sheet resolution.
