## MODIFIED Requirements

### Requirement: Streaming reader parses a workbook from a byte stream

The streaming reader SHALL parse a `.xlsx` from a readable byte stream and yield
worksheet rows incrementally, without materializing the entire workbook in
memory. It SHALL preserve cell values and styles for the streamed rows using
the same fidelity as the whole-workbook reader on the read path. It SHALL resolve
each worksheet's file from the `xl/_rels/workbook.xml.rels` target path (mapped by
`r:id`), not by parsing digits out of the sheet filename, so that worksheets with
non-default filenames or filenames whose number disagrees with document order are
still read from the correct file.

#### Scenario: Read rows incrementally from a stream

- **WHEN** the streaming reader is given a readable `.xlsx` byte stream and iterated
- **THEN** it yields each worksheet row in order with its cell values (number / string / boolean / date / formula) populated, and does not hold the full workbook in memory at once

#### Scenario: Streaming read preserves cell values

- **WHEN** a workbook written by the whole-workbook writer (or by Excel/ExcelJS) is read via the streaming reader
- **THEN** the read-back cell values equal the written values for every streamed row

#### Scenario: Resolves sheet file by rels target path

- **WHEN** a workbook's `workbook.xml.rels` maps a sheet `r:id` to a target such as `worksheets/sheet3.xml` (or a non-default name like `worksheets/sheet_v2.xml`)
- **THEN** the reader opens that exact target file via its `xl/`-prefixed path, never re-deriving the file from digits extracted from the filename

## ADDED Requirements

### Requirement: Streaming reader bounds resource usage on untrusted input

The streaming reader SHALL enforce a streaming size cap on every part it reads
(`xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, each `xl/worksheets/sheetN.xml`,
and `xl/sharedStrings.xml`). The cap SHALL bound the *actual* decompressed bytes read
from the zip entry (via a bounded reader), not merely the size declared in the zip
central directory, so that a part declaring a small uncompressed size but decompressing
to a much larger size cannot exhaust memory. The cap SHALL be `MAX_ENTRY_BYTES`
(16 MiB) per entry, and the per-sheet SAX event count SHALL be bounded by `MAX_EVENTS`
(5,000,000); both SHALL be documented as the streaming resource contract.

#### Scenario: Legitimately oversized part fails with a clear error

- **WHEN** a streamed part's declared uncompressed size exceeds `MAX_ENTRY_BYTES`
- **THEN** the reader returns a `Read` error stating the part exceeds the streaming size limit

#### Scenario: Hostile declared size cannot exceed the real bound

- **WHEN** a zip entry declares a small uncompressed size but decompresses to more than `MAX_ENTRY_BYTES`
- **THEN** the bounded reader stops at `MAX_ENTRY_BYTES` (the part is not fully read) and the reader returns an error rather than allocating the full decompressed size

#### Scenario: Excessively large sheet hits the event cap

- **WHEN** a sheet's SAX event count exceeds `MAX_EVENTS`
- **THEN** the reader returns a `Read` error stating the sheet exceeds the event limit

### Requirement: Streaming preserves empty cells distinctly from empty strings

The streaming reader/writer SHALL distinguish an empty cell (no value) from a cell
holding an empty string `""` across a round-trip. An empty cell SHALL be represented as
a distinct empty value (not as `Text("")`), and SHALL serialize with no value element
so it round-trips as empty rather than as a text cell.

#### Scenario: Empty cell round-trips as empty

- **WHEN** an empty JS cell (`{}` or `{ value: null }`) is written by the streaming writer and read back by the streaming reader
- **THEN** the read-back cell is an empty cell, not a text cell holding `""`

#### Scenario: Empty string cell round-trips as text

- **WHEN** a cell holding the empty string `""` is written and read back
- **THEN** the read-back cell is a text cell with value `""`, distinct from an empty cell
