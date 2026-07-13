# indexed-color-references Specification

## Purpose

Resolves `<color indexed="N"/>` references in OOXML styles to ARGB hex strings via the standard 56-entry system color palette. Introduced by change `v0.6.0-theme-color-references`, shipped v0.6.0.

## Requirements

### Requirement: Indexed color references resolve to ARGB on read

The style reader SHALL resolve `<color indexed="N"/>` references to ARGB via the standard 56-entry system color palette (ECMA-376 §18.8.27), honoring a custom `<indexedColors>` override from `xl/theme1.xml` when present. Resolution applies at the same three color parse sites as theme colors and stores the result in the existing `color: Option<String>` ARGB field.

#### Scenario: Standard indexed palette

- **WHEN** `xl/styles.xml` contains `<color indexed="8"/>`
- **THEN** the parsed color equals the documented ARGB for system-index 8

#### Scenario: Custom indexedColors override

- **WHEN** `theme1.xml` declares `<indexedColors>` with a non-default RGB at index 2
- **THEN** `indexed="2"` resolves to that custom ARGB

#### Scenario: Out-of-range index

- **WHEN** `indexed` ≥ 56
- **THEN** the color resolves to `None` (no panic)
