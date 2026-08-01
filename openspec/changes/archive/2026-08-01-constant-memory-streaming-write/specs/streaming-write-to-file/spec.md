## Purpose

Allows the streaming XLSX writer to emit a constant-memory `.xlsx` directly to disk.

## ADDED Requirements

### Requirement: Streaming writer can finalize directly to a file path

The streaming writer SHALL provide a `finalizeToFile(file_path: string)` method that
emits a valid `.xlsx` to the given file path on disk. The writer SHALL write each
zip file entry to disk as its sheet XML is produced, flushing to the filesystem so
that the process does not hold the full workbook XML (or the full compressed archive)
in memory at any point. At no point shall the process buffer more than one sheet's
XML worth of cell data plus the shared-strings and style accumulators.

#### Scenario: Finalize to file produces valid xlsx

- **WHEN** sheets are added incrementally to the streaming writer and `finalizeToFile("/tmp/out.xlsx")` is called
- **THEN** a valid `.xlsx` file is created at that path that round-trips through the whole-workbook reader

#### Scenario: Finalize to file is constant-memory

- **WHEN** a workbook with many sheets is written via `finalizeToFile`
- **THEN** peak process memory stays bounded by one sheet's cell data plus the shared-strings and style tables, regardless of total workbook size

#### Scenario: Finalize to file round-trips through streaming reader

- **WHEN** a workbook produced by `finalizeToFile` is read back by the streaming reader
- **THEN** the read-back rows and cell values match what was written

#### Scenario: Finalize to file rejects invalid paths

- **WHEN` `finalizeToFile` is called with a path in a non-existent directory or an unwritable location
- **THEN** the call rejects with an error (file not created or left incomplete)
