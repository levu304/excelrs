## ADDED Requirements

### Requirement: Release smoke test verifies styled round-trip

The release pipeline SHALL round-trip a styled `.xlsx` workbook through both the write and read paths and assert that cell-level style survives the read, so a read-path style-loss regression fails the release before publish.

#### Scenario: Styled workbook round-trips through the read path

- **WHEN** the release smoke test writes a workbook with a cell styled `font.bold = true` and `fill.foreground = "FFFF0000"`, then reads that workbook back from bytes
- **THEN** the read-back cell SHALL report `font.bold = true` and `fill.foreground = "FFFF0000"`, and the release job SHALL fail if either assertion is false

#### Scenario: Existing writer-only behavior is preserved

- **WHEN** the release smoke test runs the existing writer exercise (`setCellStyle` → `write()`)
- **THEN** it SHALL continue to pass, and the new read-path assertions SHALL be added on top of it rather than replacing it
