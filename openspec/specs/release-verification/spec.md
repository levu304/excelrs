# release-verification Specification

## Purpose
TBD - created by archiving change fix-issue-3-release-hardening. Update Purpose after archive.
## Requirements
### Requirement: Release smoke test verifies styled round-trip

The release pipeline SHALL round-trip a styled `.xlsx` workbook through both the write and read paths and assert that cell-level style survives the read, so a read-path style-loss regression fails the release before publish.

#### Scenario: Styled workbook round-trips through the read path

- **WHEN** the release smoke test writes a workbook with a cell styled `font.bold = true` and `fill.foreground = "FFFF0000"`, then reads that workbook back from bytes
- **THEN** the read-back cell SHALL report `font.bold = true` and `fill.foreground = "FFFF0000"`, and the release job SHALL fail if either assertion is false

#### Scenario: Existing writer-only behavior is preserved

- **WHEN** the release smoke test runs the existing writer exercise (`setCellStyle` → `write()`)
- **THEN** it SHALL continue to pass, and the new read-path assertions SHALL be added on top of it rather than replacing it

### Requirement: Release smoke test round-trips merges and row styles

The `release.yml` functional smoke test SHALL, in addition to the cell `font.bold` + `fill.foreground` round-trip, write a workbook containing a merged range and a row-level style, read it back, and assert both survive; the release job SHALL fail if either assertion is false.

#### Scenario: Merged range and row style survive the release smoke test

- **WHEN** the release smoke test writes a workbook with a merged range and a styled row, then reads it back from bytes
- **THEN** the read-back worksheet SHALL report the merged range and the row style, and the release job SHALL fail if either is missing

