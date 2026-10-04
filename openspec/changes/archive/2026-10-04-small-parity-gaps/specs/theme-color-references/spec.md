# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
