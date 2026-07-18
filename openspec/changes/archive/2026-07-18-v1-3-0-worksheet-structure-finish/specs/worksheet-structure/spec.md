# worksheet-structure Specification

## Purpose

Worksheet-structure parity with ExcelJS: row/column **outline levels** (grouping),
row/column **page breaks**, and row **insertion / splicing / duplication**
(`insertRow` / `spliceRows` / `duplicateRow`) with correct row + cell
renumbering. Introduced by change `v1-3-0-worksheet-structure-finish`.

## ADDED Requirements

### Requirement: Worksheet exposes row/column outline level (grouping)

`Row` SHALL expose an `outlineLevel` property (integer `0`–`7`, default `0`) and
`Column` SHALL expose an `outlineLevel` property (integer `0`–`7`, default `0`).
`Worksheet` SHALL emit `<row outlineLevel="N">` for rows with `outlineLevel > 0`
and `<col … outlineLevel="N"/>` for columns with `outlineLevel > 0`, and SHALL
restore both from a source workbook on read so grouping survives the round-trip.

#### Scenario: Set and read a row outline level

- **WHEN** `ws.getRow(3).outlineLevel = 2`
- **THEN** the written worksheet contains `<row r="3" outlineLevel="2">` and, after read, `ws.getRow(3).outlineLevel === 2`

#### Scenario: Set and read a column outline level

- **WHEN** the worksheet has a column with `outlineLevel = 1` set via API
- **THEN** the writer emits a `<cols>` block containing `<col … outlineLevel="1"/>` and the reader restores `column.outlineLevel === 1`

#### Scenario: Outline levels absent do not change output

- **WHEN** no row or column has `outlineLevel > 0`
- **THEN** the writer emits no `outlineLevel` attribute on `<row>` and no `<cols>` block (byte-compatible with prior versions)

### Requirement: Worksheet exposes row/column page breaks

`Worksheet` SHALL expose `rowBreaks` and `colBreaks` (each a collection of
1-indexed numbers). `Worksheet` SHALL emit `<rowBreaks>` and `<colBreaks>`
(after `sheetData` and before `pageMargins`, in schema-correct order) containing
one `<brk>` per entry, and SHALL parse them back from a source workbook. Empty
break collections SHALL emit no break elements.

#### Scenario: Set and read row breaks

- **WHEN** `ws.rowBreaks = [5, 10]`
- **THEN** the worksheet contains `<rowBreaks count="2"><brk id="5" …/><brk id="10" …/></rowBreaks>` and, after read, `ws.rowBreaks` includes `5` and `10`

#### Scenario: Set and read column breaks

- **WHEN** `ws.colBreaks = [3]`
- **THEN** the worksheet contains `<colBreaks count="1"><brk id="3" …/></colBreaks>` and, after read, `ws.colBreaks` includes `3`

#### Scenario: No breaks omits the elements

- **WHEN** `rowBreaks` and `colBreaks` are empty
- **THEN** the writer emits neither `<rowBreaks>` nor `<colBreaks>`

### Requirement: Worksheet exposes row insertion and mutation

`Worksheet` SHALL expose `insertRow(rowNumber, values?)`,
`spliceRows(start, count, rows?)`, and `duplicateRow(rowNumber, count,
includeStyle)`. These SHALL shift existing rows by renumbering every affected
`Row` and every `Cell` inside it (row number + A1 address), preserving values
and styles. Rows outside the shifted range SHALL be unaffected.

#### Scenario: insertRow shifts rows down

- **WHEN** a worksheet has values in rows 1–3 and `ws.insertRow(2, ["x"])`
- **THEN** the former row 2 becomes row 3, the former row 3 becomes row 4, a new row 2 holds `["x"]`, and all original values/styles survive at their new positions

#### Scenario: spliceRows removes and inserts

- **WHEN** a worksheet has rows 1–5 and `ws.spliceRows(2, 2, [[ "a" ], [ "b" ]])`
- **THEN** the former rows 2–3 are removed, `[ "a" ]` / `[ "b" ]` occupy rows 2–3, former rows 4–5 shift to 4–5 (renumbered), and no data is lost

#### Scenario: duplicateRow copies below with style

- **WHEN** row 2 has a value and a bold style and `ws.duplicateRow(2, 1, true)`
- **THEN** a copy of row 2 is inserted immediately below as row 3, the original row 2 is unchanged, and the copy's value + bold style are preserved

### Requirement: Round-trip fidelity for worksheet structure

`excelrs` SHALL preserve, across write then read, row/column `outlineLevel`,
`rowBreaks` / `colBreaks`, and inserted/duplicated/spliced rows (values + styles
- final positions) — whether the structure was authored by Excel or by ExcelJS.

#### Scenario: ExcelJS-authored structure round-trips

- **WHEN** grouping, page breaks, and `insertRow`/`duplicateRow` are applied via the API, written, and re-read
- **THEN** re-read `outlineLevel`, breaks, and row contents match the source

#### Scenario: Excel-authored structure round-trips

- **WHEN** an Excel-authored `.xlsx` with grouped rows/columns and manual page breaks is read, written, and re-read
- **THEN** the grouping levels and page breaks are preserved
