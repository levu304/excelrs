# diagonal-border Specification

## Purpose

Diagonal cell border round-trip: `Border.diagonal*` parsed on read and emitted on write. Write has shipped since v0.5.0; v0.12.0 adds the read side.

## Requirements

### Requirement: Reader parses diagonal borders

When reading `xl/styles.xml`, the reader SHALL parse the `<diagonal>` side and the `diagonalUp` / `diagonalDown` attributes on `<border>` into `Border.diagonal` (a `BorderStyle`) and `Border.diagonal_up` / `Border.diagonal_down` (booleans).

#### Scenario: Read a diagonal border written by excelrs

- **WHEN** a cell border was written with `diagonal: { style: "thin" }`, `diagonalUp: true`, `diagonalDown: false` and the workbook is read back
- **THEN** `cell.style.border.diagonal.style === "thin"`, `diagonal_up === true`, `diagonal_down === false`

#### Scenario: No diagonal → undefined

- **WHEN** a cell border has no diagonal
- **THEN** `cell.style.border.diagonal` is `undefined`/`null` and `diagonal_up`/`diagonal_down` are `undefined`/`null`

### Requirement: Workbook round-trips diagonal borders

A workbook written by excelrs with a diagonal border SHALL, after being read back, yield a `Border` whose diagonal fields match the originally written values.

#### Scenario: Write then read preserves diagonal

- **WHEN** a diagonal border is written and the file is read back
- **THEN** `diagonal`, `diagonal_up`, and `diagonal_down` match the written values
