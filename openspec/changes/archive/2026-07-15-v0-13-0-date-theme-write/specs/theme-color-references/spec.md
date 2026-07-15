## ADDED Requirements

### Requirement: Theme color references resolve to ARGB on write

The writer SHALL emit the resolved ARGB (`<color rgb="..."/>`) for a color that originated from a `<color theme="N"/>` reference, because downstream consumers such as ExcelJS cannot resolve `<color theme="N"/>` references back to a color. The resolved ARGB is computed at read time (theme index + optional tint), so the visual color is preserved on round-trip. The public `color` value SHALL remain the resolved ARGB string (no public API change for colors).

#### Scenario: Themed font color written back as resolved ARGB

- **WHEN** excelrs reads a file whose font color is `<color theme="4"/>` and writes it back
- **THEN** the output `styles.xml` contains `<color rgb="FF4F81BD"/>` (the resolved ARGB), so consumers like ExcelJS can read the color

#### Scenario: Themed color with tint resolves to ARGB

- **WHEN** a color is read as `<color theme="4" tint="-0.5"/>`
- **THEN** the written output is `<color rgb="..."/>` with the tint applied to the resolved ARGB

#### Scenario: Public color value unchanged (ARGB string)

- **WHEN** a themed color is read
- **THEN** `cell.style.font.color` is still the resolved ARGB string (e.g. `"FF4F81BD"`), not an object
