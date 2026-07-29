## ADDED Requirements

### Requirement: Writer filters non-anchor cells from merged ranges in sheetData

The writer SHALL omit cells whose address falls inside a declared merged range from the `<sheetData>` XML emission, unless the cell is the top-left anchor of that range.

#### Scenario: Merged range with data in every cell

- **WHEN** a worksheet has row 3 with data in columns A through L
- **AND** cells F3:K3 are merged via `mergeCells("F3:K3")`
- **AND** F3 has a thick bottom border style
- **THEN** the emitted sheetData SHALL contain `F3` with its border style index
- **AND** SHALL NOT contain `<c>` elements for `G3`, `H3`, `I3`, `J3`, or `K3`
- **AND** the `<mergeCells>` element SHALL declare `<mergeCell ref="F3:K3"/>`

#### Scenario: Merge range with only anchor cell having data

- **WHEN** a worksheet merges range `B2:D4`
- **AND** only cell B2 has a value
- **AND** B2 has no explicit style (Normal)
- **THEN** the emitted sheetData SHALL contain `<c r="B2" s="0">`
- **AND** SHALL NOT contain cells for C2, D2, B3, C3, D3, B4, C4, D4
- **AND** the `<mergeCells>` element SHALL declare `<mergeCell ref="B2:D4"/>`

#### Scenario: Cells outside merged ranges unaffected

- **WHEN** a worksheet has cells in a merged range AND cells outside any merge range
- **THEN** cells outside merge ranges SHALL be emitted with their full style as before
- **AND** the change SHALL NOT affect emission of non-merged cells

### Requirement: Helper to identify anchor cell of a merged range

The Worksheet model SHALL provide a method to determine whether a given (row, col) position is the top-left anchor cell of any declared merged range.

#### Scenario: Anchor detection

- **WHEN** merging range `F3:K3`
- **AND** checking position (row=3, col=6) (address F3)
- **THEN** the helper SHALL return true (this is the anchor)

#### Scenario: Non-anchor inside merged range

- **WHEN** merging range `F3:K3`
- **AND** checking position (row=3, col=7) (address G3)
- **THEN** the helper SHALL return false (non-anchor, should be filtered)

#### Scenario: Outside merged range

- **WHEN** merging range `F3:K3`
- **AND** checking position (row=1, col=1) (address A1)
- **THEN** the helper SHALL return false (outside merge range)
