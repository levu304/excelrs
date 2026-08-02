## Purpose

Define the observable behavior of formula evaluation added to excelrs's cell/worksheet model.
Evaluation is optional and additive: existing formula preservation behavior is unchanged unless
the `formula-eval` feature is enabled.

## ADDED Requirements

### Requirement: cell evaluation returns a computed scalar

`Cell.cachedValue` (and the Rust `Cell::evaluate`) SHALL return the computed scalar result of the
cell's formula given the current workbook state, or an error sentinel when the formula cannot be
evaluated. Inputs are the cell's `<f>` string and the referenced cells in the same workbook.

#### Scenario: numeric formula over numeric cells

WHEN a formula cell's referenced cells all contain plain numeric values
THEN reading `cachedValue` returns the numeric result of the formula

#### Scenario: referenced cell holds an error

WHEN a referenced cell contains an error value such as `#DIV/0!`
THEN `cachedValue` returns the error sentinel, not a panic

### Requirement: references resolve across sheets by name

Evaluation SHALL resolve cell references to sibling cells within the same worksheet, and SHALL
resolve references to other worksheets by sheet name.

#### Scenario: cross-sheet reference resolves to target sheet cell

WHEN a formula references `Sheet2!A1`
THEN evaluation returns the value of `Sheet2!A1` from the target worksheet

### Requirement: evaluated result is materialized as cached value on write

Upon successful evaluation, the cached scalar SHALL be persisted so the xlsx writer emits
`<f>{formula}</f><v>{computed}</v>` for the formula cell.

#### Scenario: workbook write contains computed cached value

WHEN a formula cell has been evaluated and the workbook is written to xlsx
THEN the output file contains both the original formula text in `<f>` and the computed number
in `<v>`

### Requirement: evaluation is gated behind an opt-in feature

Evaluation SHALL be unavailable unless the `formula-eval` feature is enabled at compile time;
when the feature is off, the evaluation API SHALL not be public.

#### Scenario: without the feature the API is absent

WHEN the crate is built without `formula-eval`
THEN `Cell.cachedValue` and `Cell::evaluate` are absent from the public API surface

### Requirement: streaming reader does not expose evaluation

On the streaming reader, formula cells SHALL NOT expose a computed `cachedValue`: cached `<v>` is
not retained on the streaming path, so evaluation yields no cached result.

#### Scenario: streaming read exposes formula but not computed value

WHEN a formula cell is read via the streaming API
THEN `cachedValue` is absent / null whereas the formula string remains available
