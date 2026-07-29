## MODIFIED Requirements

### Requirement: Writer filters non-anchor cells from merged ranges in sheetData

The writer SHALL omit cells whose address falls inside a declared merged range from the `<sheetData>` XML emission, unless the cell is the top-left anchor of that range.

#### Scenario: Merged range with data in every cell (regression)

- **WHEN** a worksheet has row 3 with data in columns A through L
- **AND** cells F3:K3 are merged via `mergeCells("F3:K3")`
- **AND** F3 has a thick bottom border style
- **AND** G3:K3 also have values and styles set
- **THEN** the emitted sheetData SHALL contain `F3` with its border style index
- **AND** SHALL NOT contain `<c>` elements for `G3`, `H3`, `I3`, `J3`, or `K3`
- **AND** the `<mergeCells>` element SHALL declare `<mergeCell ref="F3:K3"/>`
- **AND** cells after the merge range (L3) SHALL be emitted with correct style index

#### Scenario: Style iterator stays in sync when cells are skipped

- **WHEN** a worksheet has merged range `F3:K3` with values in G3:K3
- **AND** F3 has a border style
- **THEN** the style index for L3 (first cell after merge range) SHALL be correct (not shifted by the skipped cells)

### Requirement: Writer uses consolidated `is_cell_merged_anchor()` helper

The `write_cells_with_styles` function SHALL call `ws.is_cell_merged_anchor(cell_row, cell_col)` instead of reimplementing the merge-range containment check inline.

#### Scenario: Writer delegates anchor check to model helper

- **WHEN** `write_cells_with_styles` processes a cell at (row=3, col=7) (G3)
- **AND** the worksheet has merge range `F3:K3`
- **THEN** it calls `ws.is_cell_merged_anchor(3, 7)` which returns `false`
- **AND** the cell is skipped (not emitted in sheetData)
