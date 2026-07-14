# exceljs-parity Specification

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
