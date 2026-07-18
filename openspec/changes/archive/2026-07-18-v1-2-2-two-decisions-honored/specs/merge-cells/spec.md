## ADDED Requirements

### Requirement: Merged cell ranges survive a write to read round-trip

The reader SHALL parse `<mergeCells>` from the worksheet XML and restore the merged ranges so a workbook written with merged cells reads back with the same merges.

#### Scenario: Write then read a merged range

- **WHEN** a workbook has `ws.mergeCells("B2:D4")` applied, is written, and is read back
- **THEN** the read-back worksheet SHALL report the same merged range (`B2:D4`)

#### Scenario: Anchor cell keeps its value, non-master cells stay empty

- **WHEN** the top-left anchor cell of a merge holds a value and the workbook is written then read back
- **THEN** the read-back anchor cell SHALL retain that value and the other cells inside the range SHALL carry no phantom value

### Requirement: Query merged-range membership

The worksheet SHALL expose `isMerged(row, col)` returning the enclosing merged
range string (e.g. `"B2:D4"`) when the 1-indexed (row, col) lies inside a merged
range, or `null` otherwise. This closes the ExcelJS per-cell merged-state parity
gap without duplicating range state.

#### Scenario: Cell inside a merged range

- **WHEN** `ws.mergeCells("B2:D4")` is applied, written, and read back
- **THEN** `ws.isMerged(3, 3)` SHALL return `"B2:D4"`

#### Scenario: Cell outside any merged range

- **WHEN** the same workbook is read back
- **THEN** `ws.isMerged(1, 1)` SHALL return `null`

### Requirement: Namespace-prefixed merge cells are parsed

The reader SHALL parse `<mergeCell>` elements regardless of XML namespace prefix
(i.e. both `<mergeCell ref="A1:C3"/>` and `<x:mergeCell ref="A1:C3"></x:mergeCell>`)
so non-conformant producers are tolerated.

#### Scenario: Prefixed mergeCell element

- **WHEN** a worksheet part contains a namespace-prefixed `<x:mergeCell ref="A1:C3">`
- **THEN** the read-back worksheet SHALL report the merged range `A1:C3`
