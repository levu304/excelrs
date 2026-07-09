## MODIFIED Requirements

### Requirement: Cell-level value mutation persists to worksheet
A `Cell` obtained via `Worksheet.getCell*` or `Row.getCell*` SHALL share mutable state with the owning worksheet. Assigning `cell.value = x` on such a cell MUST persist into the worksheet's internal model and be present after a write/read round-trip.

#### Scenario: Set value on fetched cell, read back after round-trip
- **WHEN** `ws.getCell('A1').value = 42` is assigned on a worksheet, then the workbook is written and read back
- **THEN** `ws.getCell('A1').value.number` equals `42`

#### Scenario: Set value chained from row
- **WHEN** `ws.getRow(1).getCell(1).value = "hi"` is assigned
- **THEN** `ws.getCell('A1').value.string` equals `"hi"`

### Requirement: Cell-level style mutation persists to worksheet
A `Cell` obtained via `Worksheet.getCell*` or `Row.getCell*` SHALL share mutable state with the owning worksheet. Assigning `cell.style = {...}` on such a cell MUST persist into the worksheet's internal model and survive a write/read round-trip, matching exceljs chainable-mutation behavior.

#### Scenario: Set style on fetched cell, read back after round-trip
- **WHEN** `ws.getCell('B2').style = { font: { bold: true } }` is assigned, then the workbook is written and read back
- **THEN** `ws.getCell('B2').style.font.bold` is `true`

#### Scenario: Style set via getCell equals style set via setCellStyle
- **WHEN** `ws.getCell('C3').style = { fill: { kind: 'solid', foreground: 'FFFF0000' } }` and separately `ws.setCellStyle(4, 4, { font: { italic: true } })` are applied
- **THEN** both cells retain their respective styles after a write/read round-trip

#### Scenario: Clearing style via assignment resets to Normal
- **WHEN** a cell with style `{ font: { bold: true } }` has `cell.style = null` (or `{}`) assigned
- **THEN** the cell's style is `None` (Normal) and is written without a style index
