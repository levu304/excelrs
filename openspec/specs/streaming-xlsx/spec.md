# streaming-xlsx Specification

## Purpose

TBD - created by archiving change v2-0-0-streaming-parity-capstone. Update Purpose after archive.
## Requirements
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

### Requirement: Streaming writer emits a workbook to a byte stream

The streaming writer SHALL accept rows incrementally and emit a valid `.xlsx`
to a writable byte stream without buffering the whole workbook in memory.

#### Scenario: Write rows incrementally to a stream

- **WHEN** rows are added one-by-one to the streaming writer and the output stream is consumed
- **THEN** a valid `.xlsx` is produced containing exactly the added rows with their cell values preserved

#### Scenario: Streaming write round-trips through the streaming reader

- **WHEN** a workbook produced by the streaming writer is read back by the streaming reader
- **THEN** the read-back rows and cell values match what was written

### Requirement: Formula-capture state resets at cell boundary

The streaming XLSX reader SHALL reset its formula-capture state at the end of
every cell, so that a malformed or truncated cell missing its `</f>` end tag
cannot cause the next cell's value to be captured into the prior cell's formula.

#### Scenario: Missing `</f>` does not leak into the next cell

- **WHEN** a cell opens an `<f>` formula element but the corresponding `</f>` never arrives before the cell closes
- **THEN** the reader resets its formula-capture flag at the cell boundary, so the following cell's text/value is captured as that cell's own value (not appended to the prior cell's formula)

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

### Requirement: Streaming reader resolves sheet files from rels targets tolerating absolute paths

The streaming reader SHALL resolve each worksheet's XML part from the targets in
`xl/_rels/workbook.xml.rels`. It SHALL tolerate both relative targets (relative to
`xl/`) and absolute, package-rooted targets (leading `/`), resolving each to the
correct package path without a doubled `xl/` prefix.

#### Scenario: Absolute rels Target resolves to its package path

- **WHEN** a workbook's `xl/_rels/workbook.xml.rels` declares `Target="/xl/worksheets/sheet1.xml"`
- **THEN** the reader resolves the sheet at package path `xl/worksheets/sheet1.xml` (not `xl//xl/worksheets/sheet1.xml`) and reads its rows

#### Scenario: Relative rels Target resolves as before

- **WHEN** a rels `Target` is `worksheets/sheet1.xml`
- **THEN** the reader resolves `xl/worksheets/sheet1.xml` and reads its rows

### Requirement: Streaming reader resolves shared formulas

The streaming XLSX reader SHALL resolve shared formulas (`<f t="shared">`) on the
read path so that a shared-formula *member* cell yields the same translated
formula text as the whole-workbook reader, not its cached `<v>` value. The reader
SHALL collect a per-sheet table of shared formulas (keyed by `si`) from the master
cells that stream by, and SHALL translate each member's relative references by the
offset between the member cell position and the master cell position, preserving
absolute (`$A$1`) and mixed (`A$1`) references. The reader SHALL keep the
shared-formula table bounded by the number of distinct shared formulas in the
sheet and SHALL NOT materialize the whole sheet, preserving the `MAX_ENTRY_BYTES`
/ `MAX_EVENTS` streaming resource contract.

#### Scenario: Shared member returns the translated formula

- **WHEN** a worksheet has a shared formula defined at `B2` (`=A1+B1`, `si="0"`, `ref="B2:B10`) and a member cell at `B5` (`<c r="B5"><f t="shared" si="0"/></c>`)
- **THEN** the streaming reader returns `StreamValue::Formula("=A4+B4")` for `B5` (relative references shifted by the +3-row offset), matching the whole-workbook reader

#### Scenario: Shared master returns its own formula

- **WHEN** the master cell `B2` of a shared formula is read
- **THEN** the streaming reader returns `StreamValue::Formula("=A1+B1")` (offset 0, no translation)

#### Scenario: Absolute and mixed references are preserved

- **WHEN** a shared formula contains absolute (`$A$1`) or mixed (`A$1`) references
- **THEN** those references appear unchanged in the resolved member formula (only relative references shift by the offset)

#### Scenario: Non-shared formulas are unchanged

- **WHEN** a cell carries an inline (non-shared) `<f>` formula
- **THEN** the streaming reader returns its formula text exactly as before, with no translation applied

#### Scenario: Memory bounds are preserved

- **WHEN** a sheet with shared formulas is streamed
- **THEN** the reader holds only a small per-sheet `si` table (bounded by the number of distinct shared formulas), materializes no whole sheet, and still enforces `MAX_ENTRY_BYTES` / `MAX_EVENTS`

#### Scenario: Member before an unseen master resolves to no formula

- **WHEN** a shared-formula member cell appears before its master cell in document order (malformed input) and its `si` is not yet known
- **THEN** the reader emits no `Formula` for that cell (no panic, no partial state), consistent with the whole-workbook reader behavior

### Requirement: Streaming reader shifts bare column and row references in shared formulas

The streaming reader SHALL, when resolving a shared-formula *member* cell, shift
bare column references (e.g. `A`) and bare row references (e.g. `5`) in the master
formula text by the member's offset, so that the resolved text matches what the
whole-workbook (calamine) reader produces. This extends the existing shared-formula
member resolution beyond `Cell` references (`A1`) and `Cell` ranges (`A1:A3`). The
streaming reader SHALL NOT shift tokens that are not valid references (function
names such as `COLUMN`, `SUM`, and quoted strings), preserving them verbatim.

#### Scenario: Bare column reference shifts by the member offset

- **WHEN** a shared-formula master text contains a bare column reference such as `A` (e.g. `=A+B`) and the member cell is shifted one column to the right
- **THEN** the streaming reader resolves the member to `=B+C`, matching the whole-workbook reader, not the unshifted `=A+B`

#### Scenario: Bare row reference shifts by the member offset

- **WHEN** a shared-formula master text contains a bare row reference such as `5` (e.g. `=A1*5`) and the member cell is shifted one row down
- **THEN** the streaming reader resolves the member to `=A2*6`, matching the whole-workbook reader, not the unshifted `=A1*5`

#### Scenario: Function names and quoted strings stay verbatim

- **WHEN** a shared-formula master text contains a function-name token (e.g. `COLUMN`, `SUM`) or a quoted string (e.g. `"A1"`)
- **THEN** the streaming reader copies those tokens verbatim and does not attempt to shift them, identical to the whole-workbook reader

