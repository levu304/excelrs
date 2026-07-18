## ADDED Requirements

### Requirement: Release smoke test round-trips merges and row styles

The `release.yml` functional smoke test SHALL, in addition to the cell `font.bold` + `fill.foreground` round-trip, write a workbook containing a merged range and a row-level style, read it back, and assert both survive; the release job SHALL fail if either assertion is false.

#### Scenario: Merged range and row style survive the release smoke test

- **WHEN** the release smoke test writes a workbook with a merged range and a styled row, then reads it back from bytes
- **THEN** the read-back worksheet SHALL report the merged range and the row style, and the release job SHALL fail if either is missing
