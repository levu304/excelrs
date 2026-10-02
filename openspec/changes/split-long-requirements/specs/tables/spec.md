## REMOVED Requirements

### Requirement: Worksheet exposes table add/get/remove

**Reason**: Enumerated four table operations, nine option fields with defaults, and the name-uniqueness rule in one 579-character requirement.

**Migration**: Replaced by four requirements: `addTable` and its options, `getTable`, `getTables`, and `removeTable` with the uniqueness rule.

## ADDED Requirements

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