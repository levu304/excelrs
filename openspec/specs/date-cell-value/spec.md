# date-cell-value Specification

## Purpose

Defines how date-valued cells cross the FFI boundary: Excel date serials are surfaced to JavaScript as native `Date` objects rather than ISO strings, written back as serial numbers paired with an appropriate number format, and anchored to UTC to match ExcelJS behavior.

## Requirements

### Requirement: Date cell values round-trip as JS Date

The reader SHALL detect numeric cells carrying a date-like number format and
represent them as a `Date` cell value whose `value_type` is `"Date"`, bridged to a
`JS Date` across the napi boundary. A `Date` cell value written then read back
SHALL equal the original `Date` (same instant).

#### Scenario: Reading a date cell yields a JS Date

- **WHEN** a worksheet cell holds a numeric serial with a date numFmt (e.g. `yyyy-mm-dd`)
- **THEN** `cell.value` is a `JS Date` (not a number or string) and `cell.value_type === "Date"`

#### Scenario: Date survives a round-trip

- **WHEN** a `Date` is written to a cell and the workbook is read back
- **THEN** the read value is a `JS Date` equal to the original instant

### Requirement: Date values serialize to Excel serial number plus numFmt on write

When writing a `Date` cell value, the writer SHALL emit the Excel date serial
number (days since 1899-12-30, fractional part = time-of-day) as the cell value,
and SHALL assign a date number format when the cell/column has none: `yyyy-mm-dd`
for date-only values, `yyyy-mm-dd hh:mm:ss` when a non-zero time component is
present.

#### Scenario: Writing a date-only value

- **WHEN** `cell.value = new Date(2026, 0, 15)` (no time component)
- **THEN** the stored cell value is the serial number for 2026-01-15 and the cell numFmt is `yyyy-mm-dd`

#### Scenario: Writing a date-time value

- **WHEN** `cell.value = new Date(2026, 0, 15, 13, 30, 0)`
- **THEN** the stored serial includes the fractional time and the numFmt is `yyyy-mm-dd hh:mm:ss`

### Requirement: Date classification uses number-format tokens

A numeric cell SHALL be classified as a `Date` only when its number format
contains explicit date/time tokens (`y`, `m`, `d`, `h`, `s`); otherwise it SHALL
remain a `Number`.

#### Scenario: Numeric cell without date format stays a Number

- **WHEN** a numeric cell uses a plain numeric numFmt (e.g. `#,##0`)
- **THEN** `cell.value_type === "Number"`

#### Scenario: Custom date format is classified as Date

- **WHEN** a numeric cell uses a custom numFmt containing `dd/mm/yyyy`
- **THEN** `cell.value_type === "Date"`

### Requirement: Date read behavior supersedes prior string output

excelrs SHALL read date cells as a `JS Date`, superseding the prior ISO-8601 string or number form. This intended behavior change SHALL be noted in release notes.

#### Scenario: Previously-string date now reads as Date

- **WHEN** a workbook with a date-formatted numeric cell is read
- **THEN** the value is a `JS Date` (not the prior ISO-8601 string form)

### Requirement: Date cell values use the UTC-anchored Excel serial mapping

`cell.value` for a Date cell and `cell.date` SHALL convert the Excel serial using the
UTC-anchored mapping `ms = (serial - 25569) * 86400000`, and
`serial = ms / 86400000 + 25569`. This matches ExcelJS 4.4 behavior.

#### Scenario: Serial converts to a UTC instant

- **WHEN** a Date cell's Excel serial is read as a `Date`
- **THEN** the result SHALL be the UTC instant given by `ms = (serial - 25569) * 86400000`

#### Scenario: Date converts back to the original serial

- **WHEN** a `Date` is written back to a Date cell
- **THEN** the emitted serial SHALL be `ms / 86400000 + 25569`, round-tripping the input serial

#### Scenario: toISOString returns the correct value

- **WHEN** a Date cell is read and its `Date` is formatted with `toISOString()`
- **THEN** the output SHALL be the UTC-anchored instant

### Requirement: UTC-anchored dates shift the displayed day west of UTC

Because the internal `Date` represents the UTC instant of the serial, local-formatting
methods (`.toString()`, `.toLocaleDateString()`) SHALL shift the displayed day in timezones
west of UTC. This is accepted, documented behavior, not a bug.

#### Scenario: Local formatting shows the shifted day

- **WHEN** a Date cell is read and formatted with `.toLocaleDateString()` in a timezone west of UTC
- **THEN** the displayed day MAY differ from the day implied by the serial in that timezone

### Requirement: A JS Date assigned to cell.value is read as UTC milliseconds

A JS `Date` assigned to `cell.value` SHALL be interpreted by its UTC milliseconds
(`Date.prototype.getTime()`), not by its local calendar fields.

#### Scenario: Local-midnight Date uses its UTC instant

- **WHEN** a JS `Date` whose local calendar day differs from its UTC day is assigned to `cell.value`
- **THEN** the stored value SHALL be derived from the `Date`'s UTC milliseconds
