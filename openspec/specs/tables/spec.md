# tables Specification

## Purpose
Defines worksheet tables: the `ws.addTable` / `getTables` / `removeTable` API, the `xl/tables/` part and its relationship, header and totals row handling, and the table-embedded autoFilter — all round-tripped.
## Requirements
### Requirement: Worksheet addTable returns a Table handle

A `Worksheet` SHALL expose `addTable(opts)` returning a `Table` handle.

#### Scenario: addTable returns a handle

- **WHEN** a caller calls `addTable(opts)` with a valid `name`, `ref`, `columns`, and `rows`
- **THEN** it SHALL return a `Table` handle

### Requirement: Table options are name, ref, headerRow, totalsRow, columns, rows, and style

`opts` SHALL accept `name` (and optional `displayName`), `ref`, `headerRow` (bool, default
`true`), `totalsRow` (bool, default `false`), `columns` (`TableColumn[]`), `rows`
(`TableRow[]`), and optional `style` (`TableStyle`).

#### Scenario: Defaults apply when optional flags are omitted

- **WHEN** `addTable(opts)` omits `headerRow` and `totalsRow`
- **THEN** `headerRow` SHALL default to `true` and `totalsRow` SHALL default to `false`

#### Scenario: Full option set is accepted

- **WHEN** `addTable(opts)` supplies `name`, `displayName`, `ref`, `headerRow`, `totalsRow`, `columns`, `rows`, and `style`
- **THEN** all supplied fields SHALL be accepted

### Requirement: getTable returns the named Table or null

A `Worksheet` SHALL expose `getTable(name)` returning the `Table` with that name, or `null`
when no such table exists.

#### Scenario: Existing name returns its Table

- **WHEN** `getTable(name)` is called for a table on the worksheet
- **THEN** it SHALL return that `Table`

#### Scenario: Missing name returns null

- **WHEN** `getTable(name)` is called for a name not on the worksheet
- **THEN** it SHALL return `null`

### Requirement: getTables returns all tables on the worksheet

A `Worksheet` SHALL expose `getTables()` returning all tables.

#### Scenario: All tables are returned

- **WHEN** a worksheet has multiple tables and `getTables()` is called
- **THEN** it SHALL return every table on that worksheet

### Requirement: removeTable removes the named table, and names are unique per worksheet

A `Worksheet` SHALL expose `removeTable(name)` removing the named table. `name` SHALL be
unique per worksheet; a duplicate SHALL raise an error.

#### Scenario: removeTable removes the named table

- **WHEN** `removeTable(name)` is called for an existing table
- **THEN** that table SHALL no longer be returned by `getTable(name)` or `getTables()`

#### Scenario: Duplicate name raises an error

- **WHEN** `addTable(opts)` supplies a `name` already present on that worksheet
- **THEN** it SHALL raise an error

### Requirement: addTable writes header, data, and totals row values into cells

`ws.addTable(opts)` SHALL write the header row, data rows, and (when
`totalsRow` is true) the totals row into the worksheet cells covered by `ref`.
The header row SHALL be written as `rows[0]` when `headerRow` is true; the data
rows SHALL follow. `removeTable` SHALL leave the underlying cell values intact.

#### Scenario: Header and data cells populated

- **WHEN** `ws.addTable({ name: "T1", ref: "A1:C3", headerRow: true, columns: [{ name: "A" }, { name: "B" }, { name: "C" }], rows: [[1, 2, 3], [4, 5, 6]] })`
- **THEN** `ws.getCell("A1").value === "A"`, `ws.getCell("A2").value === 1`, and `ws.getCell("A3").value === 4`

#### Scenario: RemoveTable keeps cells

- **WHEN** a table's cells are populated and `ws.removeTable(name)` is called
- **THEN** the worksheet still contains the populated cell values

### Requirement: Writer emits table part and relationship

When a worksheet has tables, the writer SHALL emit `xl/tables/tableN.xml`
(`<table>` with `<autoFilter>`, `<tableColumns>`, `<tableStyleInfo>`, and
`totalsRowShown`), and register a `table` relationship in the sheet `.rels`.
A worksheet with no tables SHALL NOT emit `xl/tables/` entries or `table`
relationships for that sheet.

#### Scenario: Emit table part

- **WHEN** a worksheet has one table
- **THEN** `xl/tables/table1.xml` exists, contains `<table name="…" ref="…">`, and the sheet `.rels` contains a `table` relationship

#### Scenario: No tables omits part

- **WHEN** a worksheet has no tables
- **THEN** no `xl/tables/` entry or `table` relationship is emitted for it

### Requirement: Reader parses table part

The reader SHALL parse each sheet's table part (resolved via the sheet `.rels`)
into the `Table` model: `name`, `displayName`, `ref`, `headerRow`, `totalsRow`,
`columns` (with `totalsRowLabel`/`totalsRowFunction`), `rows`, `style`, and the
table's `autoFilter` range. A sheet without a table part SHALL leave its table
list empty.

#### Scenario: Read an Excel-authored table

- **WHEN** a sheet `.rels` references `xl/tables/table1.xml` describing a 3-column table `A1:C4`
- **THEN** `ws.getTable("…")` returns a `Table` with 3 columns and `ref === "A1:C4"`

#### Scenario: Read table with totals row

- **WHEN** a table part has `totalsRowShown="1"` with a column `totalsRowFunction="sum"`
- **THEN** the parsed `Table.totalsRow` is `true` and that column carries `totalsRowFunction === "sum"`

### Requirement: Table autoFilter is round-tripped inside the table part

A table's `<autoFilter ref="…"/>` SHALL be preserved on read and written on
emit as part of `xl/tables/tableN.xml`. The worksheet-level `<autoFilter>`
element (the `auto-filter` capability) SHALL remain independent and unaffected.

#### Scenario: Table filter survives round-trip

- **WHEN** a table carries `autoFilter ref="A1:C10"` and the workbook is written then re-read
- **THEN** the re-read `Table.autofilter_ref === "A1:C10"`

### Requirement: Round-trip fidelity for tables

excelrs SHALL preserve, across a write then read, a table's `name`, `ref`, `columns`, `rows`, `style`, and `autofilter_ref` so the re-read `Table` model matches the source, whether the table was authored by Excel or by ExcelJS (`ws.addTable`).

#### Scenario: ExcelJS-authored table round-trips

- **WHEN** `ws.addTable({ name: "T1", ref: "A1:C4", columns: [...], rows: [...] })` is written and re-read
- **THEN** the re-read table equals the source table (name, ref, columns, rows)

#### Scenario: Excel-authored table round-trips

- **WHEN** an Excel-authored `.xlsx` with a table is read, written, and re-read
- **THEN** the table's `name`, `ref`, `columns`, and `rows` are preserved
