# sheet-protection Specification

## Purpose
TBD - created by archiving change v0-11-0. Update Purpose after archive.
## Requirements
### Requirement: Worksheet exposes protection flags

A `Worksheet` SHALL expose a `protection` getter returning the current
protection descriptor (object of boolean flags, or `null` when unprotected) and
a setter accepting such an object. Setting `protection` SHALL store the flags;
setting `null`/empty SHALL mark the sheet unprotected.

#### Scenario: Set protection flags

- **WHEN** `ws.protection = { selectLockedCells: false, formatCells: true }`
- **THEN** `ws.protection.selectLockedCells === false` and `ws.protection.formatCells === true`

### Requirement: Writer emits sheetProtection

When a worksheet is protected, the writer SHALL emit a `<sheetProtection .../>`
element at the CT_Worksheet schema position (after `sheetViews`, before
`autoFilter`/`mergeCells`). Each boolean flag SHALL be emitted as
`="1"` only when true and omitted when false/absent. An unprotected sheet SHALL
NOT emit `<sheetProtection>`.

#### Scenario: Emit protection flags

- **WHEN** a worksheet has `protection = { formatCells: false, selectLockedCells: true }`
- **THEN** the sheet XML contains `<sheetProtection selectLockedCells="1"/>` and does NOT contain `formatCells`

#### Scenario: Unprotected sheet omits sheetProtection

- **WHEN** a worksheet has no protection set
- **THEN** the sheet XML SHALL NOT contain a `<sheetProtection>` element

### Requirement: Reader parses sheetProtection

The reader SHALL parse `<sheetProtection>` from `xl/worksheets/sheetN.xml` and
populate `ws.protection` with each boolean attribute resolved via the OOXML
boolean convention (`"1"`/`"true"` → true, absent or `"0"`/`"false"` → false). A
sheet without `<sheetProtection>` SHALL leave `ws.protection` unset.

#### Scenario: Read protection written by Excel or ExcelJS

- **WHEN** a sheet XML contains `<sheetProtection selectLockedCells="1" formatCells="0"/>`
- **THEN** `ws.protection.selectLockedCells === true` and `ws.protection.formatCells === false`

#### Scenario: File without sheetProtection

- **WHEN** a workbook has no `<sheetProtection>` in any sheet
- **THEN** `ws.protection` is unset for every sheet; reader does not error

