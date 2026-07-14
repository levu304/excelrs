# auto-filter Specification

## Purpose

Covers worksheet auto-filter read/write: a single `<autoFilter ref="…">`
attribute on the worksheet, mirroring ExcelJS `ws.autoFilter = "A1:C1"`.

## ADDED Requirements

### Requirement: Worksheet exposes an autoFilter property

A `Worksheet` SHALL expose an `autoFilter` getter returning the current
auto-filter range string (`Option<String>`) and a setter accepting a range
string. Setting `autoFilter` SHALL store the range; clearing it (empty string
or `null`) SHALL remove the stored range.

#### Scenario: Set and read back the filter range

- **WHEN** `ws.autoFilter = "A1:C1"`
- **THEN** `ws.autoFilter === "A1:C1"`

#### Scenario: Clearing the filter

- **WHEN** `ws.autoFilter = "A1:C1"` then `ws.autoFilter = ""`
- **THEN** `ws.autoFilter` is `null`/absent

### Requirement: Writer emits the autoFilter element

When a worksheet has an auto-filter range, the writer SHALL emit
`<autoFilter ref="{range}"/>` in the sheet XML at the CT_Worksheet schema
position (after `sheetProtection`, before `mergeCells`/`dataValidations`/`hyperlinks`).
When no range is set, the writer SHALL NOT emit an `<autoFilter>` element.

#### Scenario: Emit autoFilter for a filtered sheet

- **WHEN** a worksheet has `autoFilter === "A1:C1"`
- **THEN** the sheet XML contains `<autoFilter ref="A1:C1"/>`

#### Scenario: Empty worksheet omits autoFilter

- **WHEN** a worksheet has no auto-filter set
- **THEN** the sheet XML SHALL NOT contain an `<autoFilter>` element

### Requirement: Reader parses the autoFilter element

The reader SHALL parse `<autoFilter ref="…"/>` from `xl/worksheets/sheetN.xml`
and set `ws.autoFilter` to that range. A sheet without `<autoFilter>` SHALL
leave `ws.autoFilter` unset.

#### Scenario: Read a filter written by Excel or ExcelJS

- **WHEN** a sheet XML contains `<autoFilter ref="A1:C10"/>`
- **THEN** `ws.autoFilter === "A1:C10"`

#### Scenario: File without autoFilter

- **WHEN** a workbook has no `<autoFilter>` in any sheet
- **THEN** `ws.autoFilter` is unset for every sheet; reader does not error
