## Purpose

Defines the incremental sheet-writing behavior that makes the streaming XLSX
writer truly constant-memory for its internal intermediate buffers.

## ADDED Requirements

### Requirement: Streaming writer emits sheet XML directly without collecting all sheets first

The streaming writer SHALL write each sheet's XML directly to the zip writer as
that sheet is provided, using an inline shared-string interner and an inline
style accumulator. The writer SHALL NOT collect a per-cell emit buffer (the
former `sheet_emits`) or a per-cell style list for the entire workbook before
writing begins. `sharedStrings.xml`, `styles.xml`, and workbook metadata parts
SHALL be emitted once at finalize time, after all sheet XML has been written.

#### Scenario: Sheet XML is written during writeSheet, not deferred

- **WHEN** `writeSheet()` is called for sheet 1 and then for sheet 2
- **THEN** sheet 1's XML is written to the zip immediately during the first call and sheet 2's XML during the second, before either sharedStrings or styles parts are emitted

#### Scenario: Incremental string interning produces correct sharedStrings

- **WHEN** the same string appears in sheet 1 and sheet 2
- **THEN** `sharedStrings.xml` contains that string exactly once and both sheet XML files reference it via the same index

#### Scenario: Incremental style accumulation produces correct styles.xml

- **WHEN** the same cell style appears in sheet 1 and sheet 2
- **THEN** `styles.xml` contains that style's font/fill/border/alignment entries exactly once and the shared `cellXfs` entry once, and both sheet XML files reference the same xf ID

#### Scenario: Output is byte-identical to whole-workbook writer

- **WHEN** the same workbook data is written via the streaming writer and the whole-workbook writer
- **THEN** the resulting `.xlsx` files are byte-identical (same zip part order in central directory, same compression, same XML content)

### Requirement: Streaming writer resets sheet-level state at finalize boundary

The streaming writer SHALL reset per-sheet accumulated state (current row counter,
shared-string contributions within the sheet, style contributions within the sheet)
between sheets, while preserving cross-sheet accumulators (global shared-strings
table, global style table). The writer SHALL NOT carry residual sheet state into
the next sheet.

#### Scenario: Sheet state isolation

- **WHEN** sheet 1 has 3 rows and sheet 2 has 5 rows
- **THEN** sheet 2's XML starts row indexing from 1 and its string/style references resolve against the global (cross-sheet) tables
