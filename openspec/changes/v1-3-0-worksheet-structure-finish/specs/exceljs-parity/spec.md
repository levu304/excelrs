# exceljs-parity Specification (delta)

## MODIFIED Requirements

### Requirement: Releases consume the roadmap and update the matrix

Each `excelrs` release SHALL implement the next roadmap item(s) and update this
parity matrix to reflect the new `shipped` or `partial` status.

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
