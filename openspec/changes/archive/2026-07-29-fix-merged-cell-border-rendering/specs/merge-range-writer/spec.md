## REMOVED Requirements

### Requirement: Writer filters non-anchor cells from merged ranges in sheetData

**Reason**: Omitting non-anchor merged cells from `<sheetData>` prevents Excel from rendering the anchor cell's border and formatting across the full merged range. Excel only extends merged-cell styling when the non-anchor cells physically exist in the grid. ExcelJS — this library's drop-in compatibility target — always emits the full merge bounding box, and omitting cells produced files where merged borders appeared only under the anchor. Commit `1b8b28b` added this filter but it was a no-op for the common case (empty non-anchors were already excluded by `written_cells()`) and codified broken behavior.

**Migration**: Replaced by the new requirement "Writer emits merged range bounding box in sheetData".

## ADDED Requirements

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
