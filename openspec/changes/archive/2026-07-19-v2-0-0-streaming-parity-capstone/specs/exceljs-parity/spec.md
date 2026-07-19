## ADDED Requirements

### Requirement: excelrs declares the ExcelJS-4.4.0 v1.x parity program complete

After release v2.0.0, `excelrs` SHALL declare the ExcelJS-4.4.0 v1.x drop-in
compatibility parity program **complete**: every feature area in the v1.x
targeted roadmap (including `streaming XLSX`, and all v0.x–v1.x areas) SHALL be
marked `shipped` (or `partial` where explicitly noted), and the ROADMAP SHALL
record the program as complete. The following areas SHALL be explicitly recorded
as intentionally out of the completed program and remain `planned` / `n-a`:
charts, pivot tables, and formula evaluation (distant / deferred), plus
themes-write, sheet state (visible/hidden), tab color, and default worksheet
properties (post-v2.0.0 triage).

#### Scenario: Streaming closes the final matrix area

- **WHEN** release v2.0.0 is cut
- **THEN** the parity matrix marks `workbook IO (streams)` as `shipped`, leaving no targeted v1.x area unshipped

#### Scenario: Program declared complete with documented exclusions

- **WHEN** the v2.0.0 release is recorded
- **THEN** the ROADMAP records the v1.x drop-in ExcelJS-4.4.0 parity program as complete, and charts, pivot tables, formula evaluation, themes-write, sheet state, tab color, and default properties are listed as out of scope (`planned` / `n-a`)

## MODIFIED Requirements

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
