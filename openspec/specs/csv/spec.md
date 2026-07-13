# csv Specification

## Purpose
TBD - created by archiving change v0-9-0-csv-read-write. Update Purpose after archive.
## Requirements
### Requirement: Workbook exposes a CSV read/write handle

A `Workbook` SHALL expose a `csv` getter returning a `WorkbookCsv` handle
that shares the same underlying `Arc<Mutex<WorkbookInner>>` as the parent
workbook. The handle SHALL provide async `read(buffer)`, `readFile(path)`,
`write(opts?)`, and `writeFile(path, opts?)`.

#### Scenario: csv getter returns a shared-state handle

- **WHEN** `wb.csv` is accessed
- **THEN** a `WorkbookCsv` handle is returned that mutates the same workbook state as `wb`

### Requirement: CSV read parses into a single worksheet

`csv.read`/`readFile` SHALL parse RFC 4180 CSV text into a single Worksheet
named "Sheet1", replacing any existing worksheets. Fields that parse as a
finite `f64` SHALL become `Number` cells; all other fields SHALL become
`String` cells. An optional `{ delimiter }` SHALL override the field
separator (default `,`).

#### Scenario: Parse numbers and strings

- **WHEN** CSV `a,b\n1,hello\n2,world` is read
- **THEN** worksheet "Sheet1" has 2 data rows; row 1 has `Number(1)` in A and `String("hello")` in B; row 2 has `Number(2)` and `String("world")`

#### Scenario: Quoted fields with embedded delimiter/newline

- **WHEN** CSV `"a,b","line1\nline2"` is read
- **THEN** the first cell is `String("a,b")` and the second is `String("line1\nline2")`

#### Scenario: Custom delimiter

- **WHEN** CSV `a;b\n1;2` is read with `{ delimiter: ";" }`
- **THEN** row 1 has `String("a")` and `String("b")`; row 2 has `Number(1)` and `Number(2)`

### Requirement: CSV write serializes the first worksheet

`csv.write`/`writeFile` SHALL serialize the first worksheet
(`worksheets[0]`) to RFC 4180 CSV. Formula cells SHALL emit their cached
value when present, otherwise the raw formula string. An optional
`{ delimiter, withBom }` SHALL override the field separator (default `,`)
and prepend a UTF-8 BOM when `withBom` is true. If the workbook has no
worksheets, an empty file SHALL be written.

#### Scenario: Write numbers, strings, and formulas

- **WHEN** a worksheet has A1=`Number(1)`, B1=`String("hi")`, A2=`Formula{formula:"=1+1",value:2}`
- **THEN** the CSV is `1,hi\n2` (the formula is written as its cached value `2`)

#### Scenario: Quoting of special characters

- **WHEN** a cell contains `a,b` or a newline
- **THEN** the field is wrapped in double quotes with embedded quotes escaped as `""`

#### Scenario: Empty workbook writes empty file

- **WHEN** the workbook has zero worksheets
- **THEN** `write` returns an empty buffer

#### Scenario: Single-sheet only

- **WHEN** the workbook has 3 worksheets
- **THEN** only `worksheets[0]` is serialized; worksheets 1 and 2 are ignored

