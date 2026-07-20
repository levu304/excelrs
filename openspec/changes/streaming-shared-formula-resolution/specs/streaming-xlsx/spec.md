## ADDED Requirements

### Requirement: Streaming reader resolves shared formulas

The streaming XLSX reader SHALL resolve shared formulas (`<f t="shared">`) on the
read path so that a shared-formula *member* cell yields the same translated
formula text as the whole-workbook reader, not its cached `<v>` value. The reader
SHALL collect a per-sheet table of shared formulas (keyed by `si`) from the master
cells that stream by, and SHALL translate each member's relative references by the
offset between the member cell position and the master cell position, preserving
absolute (`$A$1`) and mixed (`A$1`) references. The reader SHALL keep the
shared-formula table bounded by the number of distinct shared formulas in the
sheet and SHALL NOT materialize the whole sheet, preserving the `MAX_ENTRY_BYTES`
/ `MAX_EVENTS` streaming resource contract.

#### Scenario: Shared member returns the translated formula

- **WHEN** a worksheet has a shared formula defined at `B2` (`=A1+B1`, `si="0"`, `ref="B2:B10`) and a member cell at `B5` (`<c r="B5"><f t="shared" si="0"/></c>`)
- **THEN** the streaming reader returns `StreamValue::Formula("=A4+B4")` for `B5` (relative references shifted by the +3-row offset), matching the whole-workbook reader

#### Scenario: Shared master returns its own formula

- **WHEN** the master cell `B2` of a shared formula is read
- **THEN** the streaming reader returns `StreamValue::Formula("=A1+B1")` (offset 0, no translation)

#### Scenario: Absolute and mixed references are preserved

- **WHEN** a shared formula contains absolute (`$A$1`) or mixed (`A$1`) references
- **THEN** those references appear unchanged in the resolved member formula (only relative references shift by the offset)

#### Scenario: Non-shared formulas are unchanged

- **WHEN** a cell carries an inline (non-shared) `<f>` formula
- **THEN** the streaming reader returns its formula text exactly as before, with no translation applied

#### Scenario: Memory bounds are preserved

- **WHEN** a sheet with shared formulas is streamed
- **THEN** the reader holds only a small per-sheet `si` table (bounded by the number of distinct shared formulas), materializes no whole sheet, and still enforces `MAX_ENTRY_BYTES` / `MAX_EVENTS`

#### Scenario: Member before an unseen master resolves to no formula

- **WHEN** a shared-formula member cell appears before its master cell in document order (malformed input) and its `si` is not yet known
- **THEN** the reader emits no `Formula` for that cell (no panic, no partial state), consistent with the whole-workbook reader behavior
