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

The writer SHALL emit the originating theme reference (`<color theme="N"/>` plus `tint` when present) for a color that originated from a `<color theme="N"/>` reference, because the theme link is now preserved end to end by the `theme1.xml` passthrough. The resolved ARGB computed at read time (theme index + optional tint) SHALL remain the public `color` value (no public API change for colors). Only a color with no originating theme reference SHALL be emitted as `<color rgb="..."/>`.

#### Scenario: Themed font color written back as resolved ARGB

- **WHEN** excelrs reads a file whose font color is `<color theme="4"/>` and writes it back
- **THEN** the output `styles.xml` contains `<color theme="4"/>` (this requirement supersedes the former resolved-ARGB emission), resolving through the passed-through theme part

#### Scenario: Themed color with tint resolves to ARGB

- **WHEN** a color is read as `<color theme="4" tint="-0.5"/>`
- **THEN** the written output is `<color theme="4" tint="-0.5"/>` with the public value still the resolved ARGB

#### Scenario: Public color value unchanged (ARGB string)

- **WHEN** a themed color is read
- **THEN** `cell.style.font.color` is still the resolved ARGB string (e.g. `"FF4F81BD"`), not an object

### Requirement: theme1.xml passes through read-to-write

The system SHALL retain the source workbook's `xl/theme/theme1.xml` part on read and SHALL emit it unchanged on write whenever the source contained one, so theme references emitted elsewhere resolve against the author's palette rather than the default scheme.

#### Scenario: Custom theme survives a round-trip

- **WHEN** a workbook containing a custom `xl/theme/theme1.xml` is read and written back
- **THEN** the output SHALL contain a byte-identical `xl/theme/theme1.xml` part

#### Scenario: Theme reference resolves against the passed-through palette

- **WHEN** a themed color is read from a custom-theme file and written back
- **THEN** the emitted theme reference SHALL resolve to the custom ARGB under that file's palette, not the default scheme

#### Scenario: Theme absent stays absent

- **WHEN** a workbook with no `xl/theme/theme1.xml` is read and written
- **THEN** the output SHALL contain no theme part and themed colors SHALL resolve against the default scheme

### Requirement: Themed and plain twin styles never share a style-table slot

The style-table builder SHALL assign separate sub-table slots to entries that differ in theme linkage, even when they resolve to the same ARGB. A themed entry (theme index plus optional tint) and a plain entry carrying the identical resolved ARGB SHALL NOT merge; each keeps its own emission form.

#### Scenario: Font twins stay separate regardless of order

- **WHEN** a workbook holds a themed font color (`theme="4"` resolving to `"FF4F81BD"`) and a plain font color `"FF4F81BD"`
- **THEN** the built style table contains two font slots, one emitted as `<color theme="4"/>` and the other as `<color rgb="FF4F81BD"/>`, whichever entry is seen first

#### Scenario: Fill twins stay separate

- **WHEN** a themed fill foreground and a plain fill foreground resolve to the same ARGB
- **THEN** they occupy two fill slots and keep their respective `theme=` / `rgb=` emission forms

#### Scenario: Single-kind workbooks are unaffected

- **WHEN** a workbook contains only themed entries (or only plain entries) with distinct values
- **THEN** style-table slot assignment is identical to before this change (no spurious splits)

