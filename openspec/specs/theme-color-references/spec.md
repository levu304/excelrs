# theme-color-references Specification

## Purpose

Resolves `<color theme="N"/>` references in OOXML styles to concrete ARGB hex strings by reading the color scheme from `xl/theme1.xml`. Introduced by change `v0.6.0-theme-color-references`, shipped v0.6.0.
## Requirements
### Requirement: Theme color references resolve to ARGB on read

The style reader SHALL resolve `<color theme="N"/>` (and optional `tint`) references found in `xl/styles.xml` to concrete ARGB hex strings, using the color scheme in `xl/theme1.xml` `<a:clrScheme>` or the OOXML default scheme when `theme1.xml` is absent. Resolution applies to font color, fill foreground/background, and all four border-side colors. The resolved value SHALL be stored in the existing `color: Option<String>` ARGB field (no model or public-API change).

#### Scenario: Font color expressed as a theme reference

- **WHEN** a workbook's `xl/styles.xml` contains `<font><color theme="4"/></font>` and `xl/theme1.xml` uses the default scheme
- **THEN** the parsed `Font.color` equals `"FF4F81BD"` (default accent1)

#### Scenario: Theme color with tint

- **WHEN** `<color theme="4" tint="-0.5"/>` is present
- **THEN** the resolved ARGB is the darkened accent1 (≈ `"FF27425E"`)

#### Scenario: theme1.xml absent

- **WHEN** the `.xlsx` has no `xl/theme1.xml`
- **THEN** resolution uses the OOXML default scheme and does not error

#### Scenario: Custom theme1.xml

- **WHEN** `theme1.xml` defines a non-default `accent1` `srgbClr`
- **THEN** `theme="4"` resolves to that custom ARGB, not the default

### Requirement: No public API change for colors

`color` SHALL remain a plain ARGB/RGB hex `string` in the napi object and `index.d.ts`. A themed file read by excelrs SHALL yield the resolved ARGB string (previously `null`), preserving exceljs drop-in compatibility.

#### Scenario: JS consumer receives ARGB string, not null

- **WHEN** excelrs reads a file whose cell font uses `theme="4"`
- **THEN** `cell.style.font.color` is the string `"FF4F81BD"` (not `null`, not an object)

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

