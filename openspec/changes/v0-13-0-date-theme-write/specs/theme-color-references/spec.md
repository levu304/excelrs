## ADDED Requirements

### Requirement: Theme color references are preserved on write

The writer SHALL emit `<color theme="N"/>` (plus `tint` when present) instead of the resolved ARGB whenever a color originated from a `<color theme="N"/>` reference, so the theme link survives a read→write round-trip. The public `color` value SHALL remain the resolved ARGB string (no public API change for colors).

#### Scenario: Themed font color written back as theme reference

- **WHEN** excelrs reads a file whose font color is `<color theme="4"/>` and writes it back
- **THEN** the output `styles.xml` contains `<color theme="4"/>` (not a flattened `rgb`)

#### Scenario: Themed color with tint preserves tint

- **WHEN** a color is read as `<color theme="4" tint="-0.5"/>`
- **THEN** the written output is `<color theme="4" tint="-0.5"/>`

#### Scenario: Public color value unchanged (ARGB string)

- **WHEN** a themed color is read
- **THEN** `cell.style.font.color` is still the resolved ARGB string (e.g. `"FF4F81BD"`), not an object
