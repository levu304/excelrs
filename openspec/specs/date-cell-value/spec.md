# date-cell-value Specification

## Purpose
TBD - created by archiving change v0-13-0-date-theme-write. Update Purpose after archive.
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

