## REMOVED Requirements

### Requirement: Parity matrix covers the ExcelJS feature areas

**Reason**: A single flat enumeration of the ExcelJS feature areas exceeded the 500-character guideline.

**Migration**: Replaced by "Parity matrix covers the ExcelJS feature areas", which preserves the same area list across scenarios.

### Requirement: excelrs declares the ExcelJS-4.4.0 v1.x parity program complete

**Reason**: Combined the historical completion record, the standing status rule, and the v2.0.0 exclusion list into one over-long requirement.

**Migration**: Replaced by "excelrs declares the ExcelJS-4.4.0 v1.x parity program complete" and "v2.0.0 parity program exclusions".

## ADDED Requirements

### Requirement: Parity matrix enumerates the ExcelJS feature areas

The matrix SHALL enumerate, at minimum, these ExcelJS feature areas:

- **Workbook IO**: xlsx, csv, streams
- **Worksheet structure**: rows, columns, cells, merge, freeze panes, auto-filter
- **Cell values and types**
- **Styling**: font, fill, border, alignment, number-format, gradient fills, diagonal borders
- **Defined names**
- **Data validation**
- **Hyperlinks**
- **Rich text**
- **Comments**
- **Images**
- **Charts**
- **Pivot tables**
- **Tables**
- **Conditional formatting**
- **Sheet and workbook protection**
- **Page setup and print**
- **Workbook views and properties**
- **Themes**

#### Scenario: Matrix lists every required feature area

- **WHEN** the parity matrix is read
- **THEN** it SHALL contain a row for each listed feature area, at minimum

#### Scenario: Styling sub-areas are individually tracked

- **WHEN** the styling row set is read
- **THEN** font, fill, border, alignment, number-format, gradient fills, and diagonal borders SHALL each appear

### Requirement: The ExcelJS-4.4.0 v1.x parity program is recorded complete at v2.0.0

The v1.x drop-in ExcelJS-4.4.0 parity program is **complete** as of release v2.0.0: every
feature area in the v1.x targeted roadmap (including `streaming XLSX`, and all v0.x-v1.x
areas) was marked `shipped` (or `partial` where explicitly noted), and the ROADMAP records
the program as complete.

This is a historical record, not a standing constraint. An area's status SHALL be whatever
the parity matrix currently says, and a later release that ships a previously-excluded area
SHALL advance that area's status without needing to amend this requirement.

#### Scenario: Matrix status is authoritative over the historical record

- **WHEN** a feature area's matrix status differs from what the v2.0.0 record described
- **THEN** the matrix status SHALL be treated as the area's current status

#### Scenario: Newly shipped area advances without amending the record

- **WHEN** a later release ships an area excluded at v2.0.0
- **THEN** the area's matrix status SHALL advance without amending the v2.0.0 completion requirement

### Requirement: v2.0.0 parity program exclusions

The v2.0.0 parity completion record SHALL list these areas as excluded from the completed
v1.x program: charts, pivot tables, and formula evaluation (distant / deferred), plus
themes-write, sheet state (visible/hidden), tab color, and default worksheet properties
(deferred to post-v2.0.0 triage).

#### Scenario: Excluded areas were distant or deferred at v2.0.0

- **WHEN** the v2.0.0 parity record is read
- **THEN** charts, pivot tables, and formula evaluation SHALL be recorded as distant or deferred

#### Scenario: Post-v2.0.0 deferrals are named

- **WHEN** the v2.0.0 parity record is read
- **THEN** themes-write, sheet state, tab color, and default worksheet properties SHALL be recorded as deferred to post-v2.0.0 triage