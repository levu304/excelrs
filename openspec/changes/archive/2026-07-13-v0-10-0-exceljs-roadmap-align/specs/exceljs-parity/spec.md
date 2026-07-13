# exceljs-parity Specification

## Purpose

Tracks `excelrs`'s feature parity with [ExcelJS](https://github.com/exceljs/exceljs) and governs how the porting roadmap is derived, prioritized, and consumed by releases. This is the contract that future releases MODIFY to record newly shipped/partial areas. Introduced by change `v0-10-0-exceljs-roadmap-align`.

## ADDED Requirements

### Requirement: excelrs maintains an ExcelJS feature-parity matrix

`excelrs` SHALL maintain a feature-parity matrix that maps each ExcelJS feature area to exactly one status: `shipped`, `partial`, `planned`, or `n-a` (explicitly out of scope). The matrix SHALL be derived by comparing `excelrs`'s actually-implemented behavior (verified against `openspec/specs/*`, `CHANGELOG.md`, and source) against the ExcelJS documented API surface.

#### Scenario: Matrix reflects a shipped area

- **WHEN** the parity matrix is generated
- **THEN** `defined-names` is marked `shipped` (released v0.7.0) with evidence from `CHANGELOG.md`

#### Scenario: Matrix reflects a not-yet-ported area

- **WHEN** the parity matrix is generated
- **THEN** feature areas with no implementation (e.g., charts) are marked `planned` or `n-a`, never `shipped`

### Requirement: Parity matrix covers the ExcelJS feature areas

The matrix SHALL enumerate, at minimum, these ExcelJS feature areas: workbook IO (xlsx / csv / streams), worksheet structure (rows / columns / cells / merge / freeze panes / auto-filter), cell values & types, styling (font / fill / border / alignment / number-format / gradient fills / diagonal borders), defined names, data validation, hyperlinks, rich text, comments, images, charts, pivot tables, tables, conditional formatting, sheet & workbook protection, page setup / print, workbook views & properties, themes.

#### Scenario: Every enumerated area has a status

- **WHEN** the matrix is generated
- **THEN** each area in the enumerated list carries one of `shipped` / `partial` / `planned` / `n-a`

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
