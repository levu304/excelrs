## ADDED Requirements

### Requirement: Row-level style survives a write to read round-trip

The reader SHALL parse the `<row s="N">` attribute and restore `Row.style` so a row styled on write reads back with the same style.

#### Scenario: Write then read a styled row

- **WHEN** a row is given a style (for example a bold font or a fill) on write and the workbook is read back
- **THEN** `Row.style` on the read-back row SHALL equal the style applied on write
