# dimension-properties Specification

## Purpose
TBD - created by archiving change fix-column-width-row-height. Update Purpose after archive.
## Requirements
### Requirement: Column widths are emitted in XLSX output

The XLSX writer SHALL include the `width` attribute on `<col>` elements when a column has an explicit width set.

#### Scenario: Write column widths to XLSX

- **WHEN** `setColumns([{width: 15.83}, {width: 14.67}])` is called and the worksheet is written to XLSX
- **THEN** the `<cols>` block contains `<col min="1" max="1" width="15.83"/>` and `<col min="2" max="2" width="14.67"/>`

#### Scenario: Column width survives write/read round-trip

- **WHEN** column widths are set, the workbook is written to XLSX, and then read back
- **THEN** `Column.width` on the read-back worksheet equals the values that were set

### Requirement: Row heights are emitted in XLSX output

The XLSX writer SHALL include the `ht` attribute on `<row>` elements when a row has an explicit height set.

#### Scenario: Write row heights to XLSX

- **WHEN** `getRow(1).height = 29` and `getRow(2).height = 14` are called and the worksheet is written to XLSX
- **THEN** the `<row>` elements include `ht="29"` and `ht="14"` respectively

#### Scenario: Row height survives write/read round-trip

- **WHEN** row heights are set, the workbook is written to XLSX, and then read back
- **THEN** `Row.height` on the read-back worksheet equals the values that were set

### Requirement: Non-grouped columns with widths are included in `<cols>`

The XLSX writer SHALL emit `<cols>` for columns that have explicit width or hidden state, regardless of whether they have an outline level set.

#### Scenario: Write ungrouped columns with widths

- **WHEN** `setColumns([{width: 10}, {width: 20}])` is called with no `outlineLevel` set
- **THEN** the `<cols>` block is emitted with both columns including their `width` attributes

