# merge-range-writer Specification

## Purpose

Defines the write side of merged cells: emitting each merged range's bounding box into `sheetData`, and the consolidated helper that identifies a range's anchor cell so only the anchor carries the value and style.

## Requirements

### Requirement: Writer emits merged range bounding box in sheetData

The writer SHALL emit every cell within a declared merged range's bounding box into `<sheetData>`, including non-anchor cells, each carrying its effective style (the anchor's style, or Normal / column style for empty non-anchors). This matches ExcelJS output so Excel renders the anchor's borders and formatting across the entire merged range.

#### Scenario: Merged range with only anchor cell having data

- **WHEN** a worksheet merges range `B2:D4`
- **AND** only cell `B2` has a value (Normal style)
- **THEN** the emitted sheetData SHALL contain `<c>` elements for `B2`, `C2`, `D2`, `B3`, `C3`, `D3`, `B4`, `C4`, `D4`
- **AND** the non-anchor cells SHALL be emitted with their effective style (Normal, no explicit border)

#### Scenario: Merged range with border on anchor

- **WHEN** a worksheet merges `F3:K3`
- **AND** `F3` has a thick bottom border style
- **THEN** the emitted sheetData SHALL contain `F3` through `K3` (all six cells)
- **AND** `F3` SHALL carry its border style index
- **AND** Excel SHALL render the thick bottom border across the full `F3:K3` width because the non-anchor cells are present in `<sheetData>`

#### Scenario: Merged range with data in every cell

- **WHEN** a worksheet has row 3 with data in columns `A` through `L`
- **AND** cells `F3:K3` are merged via `mergeCells("F3:K3")`
- **AND** every cell `F3`..`K3` has its own value and style
- **THEN** the emitted sheetData SHALL contain `F3`..`K3` with each cell's own style index
- **AND** SHALL NOT drop any non-anchor cell

#### Scenario: Non-anchor cell with its own style

- **WHEN** a worksheet merges `F3:K3`
- **AND** `F3` (anchor) has a border style
- **AND** non-anchor `G3` has its own (different) style
- **THEN** both `F3` and `G3` SHALL be emitted with their respective effective styles (matches ExcelJS)

#### Scenario: Cell outside any merged range unaffected

- **WHEN** a worksheet has cells both inside and outside merged ranges
- **THEN** cells outside merge ranges SHALL be emitted with their full style as before
- **AND** the change SHALL only add previously-omitted non-anchor merged cells
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

### Requirement: Writer uses consolidated `is_cell_merged_anchor()` helper

The `write_cells_with_styles` function SHALL call `ws.is_cell_merged_anchor(cell_row, cell_col)` instead of reimplementing the merge-range containment check inline.

#### Scenario: Writer delegates anchor check to model helper

- **WHEN** `write_cells_with_styles` processes a cell at (row=3, col=7) (G3)
- **AND** the worksheet has merge range F3:K3
- **THEN** it calls `ws.is_cell_merged_anchor(3, 7)` which returns false
- **AND** the writer emits G3 as a synthetic empty cell (no s attribute, Normal/0)
- **AND** the anchor F3 still carries its border style
