# gradient-fill Specification

## Purpose

Gradient cell fill round-trip: `Fill` gradient fields parsed on read and emitted on write. Write has shipped since v0.5.0; v0.12.0 adds the read side.

## Requirements

### Requirement: Reader parses gradient fills

When reading `xl/styles.xml`, the reader SHALL parse `<gradientFill>` (linear or path) into the `Fill` gradient fields: `kind === "gradient"`, `gradient_type` (`"linear"` | `"path"`), `gradient_degree` (linear), path geometry (`gradient_left`/`gradient_right`/`gradient_top`/`gradient_bottom` for path), and `gradient_stops` (`Vec<GradientStop>` with `color` and `position`).

#### Scenario: Read a linear gradient written by excelrs

- **WHEN** a cell style was written with a linear gradient (`gradient_type: "linear"`, `gradient_degree: 90`, two stops) and the workbook is read back
- **THEN** `cell.style.fill.kind === "gradient"`, `gradient_type === "linear"`, `gradient_degree === 90`, and `gradient_stops` equals the two written stops

#### Scenario: No gradient → Normal fill

- **WHEN** a cell has a solid or no fill
- **THEN** `cell.style.fill.kind` is not `"gradient"` and `gradient_stops` is `undefined`/`null`

### Requirement: Workbook round-trips gradient fills

A workbook written by excelrs with a gradient fill SHALL, after being read back, yield a `Fill` whose gradient fields match the originally written values.

#### Scenario: Write then read preserves gradient

- **WHEN** a gradient fill is written and the file is read back
- **THEN** the gradient `kind`, `gradient_type`, degree/path geometry, and stops match the written values
