# Style Setter Type — TypeScript Type Specification

## ADDED Requirements

### Requirement: Cell.style setter is typed `Style | null | undefined`

The `Cell.style` setter must accept `Style | null | undefined` instead of `any`.

#### Scenario: cell.style receives a valid Style object

- **WHEN** a validated `Style` object is assigned to `cell.style`
- **THEN** the style is stored and reflected in the subsequent `cell.style` getter

#### Scenario: cell.style receives null or undefined

- **WHEN** `null` or `undefined` is assigned to `cell.style`
- **THEN** the cell style resets to Normal (None), matching current behavior

#### Scenario: cell.style receives an empty object

- **WHEN** `{}` is assigned to `cell.style`
- **THEN** the cell style resets to Normal (None), matching current behavior

#### Scenario: cell.style receives an invalid Style

- **WHEN** a style object with invalid fields (e.g. bad color hex, NaN font size) is assigned to `cell.style`
- **THEN** an `ExcelrsError::InvalidStyle` error is thrown, matching current behavior

### Requirement: Row.style setter is typed `Style | null | undefined`

The `Row.style` setter must accept `Style | null | undefined` instead of `any`.

#### Scenario: row.style receives a valid Style object

- **WHEN** a validated `Style` object is assigned to `row.style`
- **THEN** the style is stored and reflected in the subsequent `row.style` getter

#### Scenario: row.style receives null

- **WHEN** `null` is assigned to `row.style`
- **THEN** the row style resets to Normal (None)

### Requirement: Column.style setter is typed `Style | null | undefined`

The `Column.style` setter must accept `Style | null | undefined` instead of `any`.

#### Scenario: column.style receives a valid Style object

- **WHEN** a validated `Style` object is assigned to `column.style`
- **THEN** the style is stored and reflected in the subsequent `column.style` getter

#### Scenario: column.style receives null

- **WHEN** `null` is assigned to `column.style`
- **THEN** the column style resets to Normal (None)

### Requirement: Worksheet.setCellStyle accepts `Style | null | undefined`

The `Worksheet.setCellStyle(row, col, style)` third parameter must accept `Style | null | undefined` instead of `any`.

#### Scenario: setCellStyle receives a valid Style

- **WHEN** `worksheet.setCellStyle(1, 1, validStyle)` is called
- **THEN** the cell at (1, 1) receives the style, matching current behavior

#### Scenario: setCellStyle receives null

- **WHEN** `worksheet.setCellStyle(1, 1, null)` is called
- **THEN** the cell at (1, 1) resets to Normal (None)
