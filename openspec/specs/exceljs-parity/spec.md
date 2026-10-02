# exceljs-parity Specification

## Purpose

Tracks `excelrs`'s feature parity with [ExcelJS](https://github.com/exceljs/exceljs) and governs how the porting roadmap is derived, prioritized, and consumed by releases. This is the contract that future releases MODIFY to record newly shipped/partial areas. Introduced by change `v0-10-0-exceljs-roadmap-align`.

## Requirements

### Requirement: excelrs maintains an ExcelJS feature-parity matrix

`excelrs` SHALL maintain a feature-parity matrix that maps each ExcelJS feature area to exactly one status: `shipped`, `partial`, `planned`, or `n-a` (explicitly out of scope). The matrix SHALL be derived by comparing `excelrs`'s actually-implemented behavior (verified against `openspec/specs/*`, `CHANGELOG.md`, and source) against the ExcelJS documented API surface.

#### Scenario: Matrix reflects a shipped area

- **WHEN** the parity matrix is generated
- **THEN** `defined-names` is marked `shipped` (released v0.7.0) with evidence from `CHANGELOG.md`

#### Scenario: Matrix reflects a not-yet-ported area

- **WHEN** the parity matrix is generated
- **THEN** feature areas with no implementation (e.g., charts) are marked `planned` or `n-a`, never `shipped`

### Requirement: Parity matrix enumerates workbook, worksheet, and cell feature areas

The matrix SHALL enumerate, at minimum, these ExcelJS feature areas: **Workbook IO** (xlsx,
csv, streams), **Worksheet structure** (rows, columns, cells, merge, freeze panes,
auto-filter), and **Cell values and types**.

#### Scenario: Workbook IO is tracked

- **WHEN** the parity matrix is read
- **THEN** it SHALL contain a row for Workbook IO covering xlsx, csv, and streams

#### Scenario: Worksheet structure is tracked

- **WHEN** the parity matrix is read
- **THEN** it SHALL contain a row for Worksheet structure covering rows, columns, cells, merge, freeze panes, and auto-filter

#### Scenario: Cell values and types are tracked

- **WHEN** the parity matrix is read
- **THEN** it SHALL contain a row for Cell values and types

### Requirement: Parity matrix enumerates styling and per-cell data feature areas

The matrix SHALL enumerate, at minimum, these ExcelJS feature areas: **Styling** (font, fill,
border, alignment, number-format, gradient fills, diagonal borders), **Defined names**,
**Data validation**, **Hyperlinks**, **Rich text**, and **Comments**.

#### Scenario: Styling sub-areas are individually tracked

- **WHEN** the styling row set is read
- **THEN** font, fill, border, alignment, number-format, gradient fills, and diagonal borders SHALL each appear

#### Scenario: Per-cell data feature areas are tracked

- **WHEN** the parity matrix is read
- **THEN** it SHALL contain rows for Defined names, Data validation, Hyperlinks, Rich text, and Comments

### Requirement: Parity matrix enumerates embedded-object and workbook-level feature areas

The matrix SHALL enumerate, at minimum, these ExcelJS feature areas: **Images**, **Charts**,
**Pivot tables**, **Tables**, **Conditional formatting**, **Sheet and workbook protection**,
**Page setup and print**, **Workbook views and properties**, and **Themes**.

#### Scenario: Embedded-object feature areas are tracked

- **WHEN** the parity matrix is read
- **THEN** it SHALL contain rows for Images, Charts, Pivot tables, and Tables

#### Scenario: Workbook-level feature areas are tracked

- **WHEN** the parity matrix is read
- **THEN** it SHALL contain rows for Conditional formatting, Sheet and workbook protection, Page setup and print, Workbook views and properties, and Themes

### Requirement: The ExcelJS-4.4.0 v1.x parity program is recorded complete at v2.0.0

The v1.x drop-in ExcelJS-4.4.0 parity program is **complete** as of release v2.0.0: every
feature area in the v1.x targeted roadmap (including `streaming XLSX`, and all v0.x-v1.x
areas) was marked `shipped` (or `partial` where explicitly noted), and the ROADMAP SHALL
record the program as complete. This requirement is a historical record.

#### Scenario: v2.0.0 records the program complete

- **WHEN** the v2.0.0 parity record is read
- **THEN** every feature area in the v1.x targeted roadmap SHALL be recorded as `shipped`, or `partial` where explicitly noted

### Requirement: Parity matrix status is authoritative over the historical record

An area's status SHALL be whatever the parity matrix currently says, and a later release that
ships a previously-excluded area SHALL advance that area's status without needing to amend
the v2.0.0 completion record.

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

### Requirement: Roadmap prioritizes unported features

From areas marked `partial` or `planned`, `excelrs` SHALL produce an ordered porting roadmap. Prioritization SHALL weigh (a) contribution to the drop-in ExcelJS compatibility promise and (b) relative implementation effort, each on a coarse `high` / `med` / `low` scale. The roadmap SHALL assign each prioritized item to a target release (e.g., v0.11.0+).

#### Scenario: Higher-value, lower-effort items come first

- **WHEN** the roadmap is generated
- **THEN** an area with `high` compat value and `low` effort is sequenced before an area with `low` compat value and `high` effort

### Requirement: Releases consume the roadmap and update the matrix

Each `excelrs` release SHALL implement the next roadmap item(s) and update this parity matrix to reflect the new `shipped` or `partial` status.

#### Scenario: Status advances on release

- **WHEN** a release ships a previously `planned`/`partial` area
- **THEN** that area's matrix status moves to `shipped` (or `partial` if only partially covered)

#### Scenario: v0.11.0 ships the quick-win worksheet features

- **WHEN** release v0.11.0 is cut
- **THEN** the matrix marks `hyperlinks` (read), `auto-filter`, freeze panes, and sheet protection as `shipped`, advancing each from `planned`

#### Scenario: v0.12.0 ships the rich-content read round-trip

- **WHEN** release v0.12.0 is cut
- **THEN** the matrix marks `rich-text`, `gradient fill`, and `diagonal border` as `shipped`, advancing each from `partial`

#### Scenario: v1.0.0 ships full worksheet & workbook parity

- **WHEN** release v1.0.0 is cut
- **THEN** the matrix marks `comments`, `images`, `page setup / print`, `headers/footers`, and `workbook views & properties` as `shipped`, advancing each from `planned`

#### Scenario: v1.1.0 ships worksheet tables

- **WHEN** release v1.1.0 is cut
- **THEN** the matrix marks `tables` as `shipped`, advancing it from `planned`/`targeted`

#### Scenario: v1.2.0 ships conditional formatting

- **WHEN** release v1.2.0 is cut
- **THEN** the matrix marks `conditional formatting` as `shipped`, advancing it from `targeted`

#### Scenario: v1.3.0 ships worksheet-structure parity finish

- **WHEN** release v1.3.0 is cut
- **THEN** the matrix marks the remaining v1.x `planned` rows — `insert/splice/duplicate rows`, `row/col outlineLevel (grouping)`, and `row/col page breaks` — as `shipped`, advancing each from `planned`

#### Scenario: v2.0.0 ships streaming XLSX and completes the parity program

- **WHEN** release v2.0.0 is cut
- **THEN** the matrix marks `workbook IO (streams)` as `shipped` and the ROADMAP records the v1.x drop-in ExcelJS-4.4.0 parity program as complete, with charts, pivot tables, formula evaluation, themes-write, sheet state, tab color, and default properties listed as out of scope

### Requirement: Parity matrix rows reflect shipped behavior

The ROADMAP parity matrix SHALL record, for each feature area, the status that the
implemented behavior actually has. An area whose behavior is implemented and round-trip
verified SHALL NOT be listed as `planned` or as "Not implemented". When a release ships an
area, the matrix row for that area SHALL be updated in the same change that ships it.

#### Scenario: Shipped area is not marked not-implemented

- **WHEN** an area's read and write paths are implemented and round-trip verified
- **THEN** the parity matrix SHALL mark that area `shipped` and SHALL NOT describe it as "Not implemented"

#### Scenario: Shipping a change updates its matrix row

- **WHEN** a change ships behavior for a feature area listed in the parity matrix
- **THEN** that change SHALL update the corresponding matrix row to the new status

### Requirement: excelrs preserves ExcelJS-compat getCell overloads

`excelrs` SHALL expose ExcelJS-compatible `getCell` overloads on `Worksheet` and `Row` that delegate to the native `getCellBy*` Rust APIs. The overloads SHALL survive `napi build` — they SHALL be re-injected through a build-time hook, never hand-patched into the generated `index.js` / `index.d.ts`.

#### Scenario: Worksheet.getCell resolves by A1 address

- **WHEN** a consumer calls `worksheet.getCell("A1")`
- **THEN** it returns the cell via the native `getCellByAddress` API

#### Scenario: Worksheet.getCell resolves by row and column

- **WHEN** a consumer calls `worksheet.getCell(2, 3)`
- **THEN** it returns the cell via the native `getCellByRc` API

#### Scenario: Row.getCell resolves by column number

- **WHEN** a consumer calls `row.getCell(5)`
- **THEN** it returns the cell via the native `getCellByColNum` API

#### Scenario: Row.getCell resolves by column letter

- **WHEN** a consumer calls `row.getCell("E")`
- **THEN** it returns the cell via the native `getCellByColLetter` API

#### Scenario: Glue survives napi build

- **WHEN** `napi build` regenerates `index.js` / `index.d.ts`
- **THEN** the `getCell` overloads are re-injected automatically and no manual re-patch is required
