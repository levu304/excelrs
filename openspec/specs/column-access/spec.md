# column-access Specification

## Purpose

Lets ExcelJS migrants size, hide, and style individual worksheet columns through a live per-column handle instead of replacing the whole column definition at once.

## Requirements
### Requirement: getColumn by number returns a live column handle

The system SHALL resolve `getColumn` with a 1-indexed column number to the worksheet's column at that position, creating the column definition when none exists. Mutations made through the returned handle SHALL persist into the worksheet model and SHALL be visible to later reads of the same column.

#### Scenario: Existing column resolves by number

- **WHEN** `getColumn(2)` is called on a worksheet with a column 2 definition
- **THEN** the returned handle SHALL expose column 2's header, key, width, hidden, style, and outline level

#### Scenario: Absent column is auto-created

- **WHEN** `getColumn(5)` is called on a worksheet with no column 5 definition
- **THEN** a column 5 definition SHALL be created and the returned handle SHALL expose it

#### Scenario: Width mutation through the handle persists

- **WHEN** `getColumn(1).width` is set to `20` and the worksheet is written to XLSX
- **THEN** the emitted `<cols>` block SHALL carry the width for column 1 and a later `getColumn(1).width` read SHALL return `20`

### Requirement: getColumn by letter resolves to the same column as by number

The system SHALL resolve `getColumn` with a column letter to the same column definition as the equivalent 1-indexed number, returning a live handle with identical mutation semantics.

#### Scenario: Letter and number reach the same column

- **WHEN** `getColumn('B')` and `getColumn(2)` are called on the same worksheet
- **THEN** both handles SHALL expose and mutate the same column 2 definition

#### Scenario: Hidden mutation through a letter handle persists

- **WHEN** `getColumn('C').hidden` is set to `true`
- **THEN** `getColumn(3).hidden` SHALL return `true`

### Requirement: Column handle mutations apply to cells without explicit style

Style, hidden, and outline-level mutations made through a column handle SHALL take the same effect as the equivalent `setColumns` definition: cells in that column with no explicit cell-level style SHALL inherit the column style on write.

#### Scenario: Bold set through handle reaches unstyled cells

- **WHEN** `getColumn(1).style` is set to a bold font and a row holding an unstyled `A1` is written
- **THEN** the emitted cell SHALL carry the bold style and explicitly styled cells SHALL keep their own style
