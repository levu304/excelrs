## REMOVED Requirements

### Requirement: Streaming reader parses a workbook from a byte stream

**Reason**: Combined the incremental-yield contract, the read-fidelity contract, and the rels-based sheet-resolution contract into one over-long requirement.

**Migration**: Replaced by three requirements: the incremental parse contract, the read-fidelity contract, and rels-based sheet resolution.

### Requirement: Streaming writer emits a workbook to a byte stream

**Reason**: At 1,959 characters this was the corpus's longest requirement. It bundled the two-phase memory model, three separate output-target contracts with distinct memory semantics, and the deferred-scope disclaimer.

**Migration**: Replaced by "Streaming writer emits a workbook to a byte stream" (the two-phase model, now stated once for the whole streaming cluster), "Streaming writer finalize targets", and the per-target requirements in this delta's ADDED block.

### Requirement: Streaming reader bounds resource usage on untrusted input

**Reason**: Combined the per-part cap coverage, the decompressed-vs-declared-size rule, the `MAX_ENTRY_BYTES` value, and the `MAX_EVENTS` value into one over-long requirement.

**Migration**: Replaced by three requirements: per-part size caps, `MAX_ENTRY_BYTES`, and `MAX_EVENTS`.

### Requirement: Streaming reader resolves shared formulas

**Reason**: Combined shared-formula collection, member translation, absolute-reference preservation, and the memory-bound constraint into one over-long requirement.

**Migration**: Replaced by three requirements: shared-formula collection, member reference translation, and bounded shared-formula tables.

### Requirement: Streaming reader shifts bare column and row references in shared formulas

**Reason**: Combined the bare-reference shifting rule with the non-reference-token preservation rule into one over-long requirement.

**Migration**: Replaced by "Streaming reader shifts bare column and row references in shared formulas" and "Streaming reader preserves non-reference tokens in shared formulas".

## ADDED Requirements

### Requirement: Streaming reader parses a workbook from a byte stream incrementally

The streaming reader SHALL parse a `.xlsx` from a readable byte stream and yield worksheet
rows incrementally, without materializing the entire workbook in memory.

#### Scenario: Rows arrive incrementally from a stream

- **WHEN** a workbook is read from a readable byte stream
- **THEN** worksheet rows SHALL be yielded as they are parsed, without the whole workbook being held in memory

### Requirement: Streaming reader preserves values and styles at whole-workbook fidelity

The streaming reader SHALL preserve cell values and styles for the streamed rows using the
same fidelity as the whole-workbook reader on the read path.

#### Scenario: Streamed rows match whole-workbook read results

- **WHEN** a workbook is read through the streaming reader
- **THEN** the streamed rows' cell values and styles SHALL match those the whole-workbook reader produces for the same workbook

### Requirement: Streaming reader resolves sheet files from rels targets

The streaming reader SHALL resolve each worksheet's file from the
`xl/_rels/workbook.xml.rels` target path mapped by `r:id`, not by parsing digits out of the
sheet filename.

#### Scenario: Sheet with a non-default filename is read correctly

- **WHEN** a worksheet's filename number disagrees with document order
- **THEN** the reader SHALL read that worksheet from the file its rels target names

### Requirement: Streaming writer buffers input sheets and streams the output phase

The streaming writer SHALL emit a valid `.xlsx` to a writable byte stream.

This requirement states the writer's two-phase memory model for the whole streaming cluster.
The `streaming-write-to-file` and `streaming-write-to-readable` capabilities cross-reference
it rather than restating it.

**Input phase:** sheets pushed via `writeSheet()` are accumulated in the `StreamWriter`
handle (`sheets: Vec<StreamSheet>`) before any zip entry is written. Peak memory for this
phase is **O(all sheets)**, NOT constant. True incremental `writeSheet()` — each sheet's XML
written to the zip as it arrives, before `finalize` — is **deferred**; see
`openspec/specs/streaming-write-incremental/spec.md` and
`docs/adr/005-streaming-write-buffering.md`.

**Output phase:** `finalize`, `finalizeToFile`, and `finalizeToReadable` emit the accumulated
sheets directly to the zip writer, writing each sheet's XML as it is produced and piping
compressed bytes through a bounded mpsc channel (cap 16). `sharedStrings.xml`, `styles.xml`,
and workbook metadata parts are emitted once at finalize time, after all sheet XML has been
written. Peak memory for this phase is constant — one sheet's XML plus the shared-strings and
style accumulators.

#### Scenario: Input phase buffers all sheets

- **WHEN** sheets are pushed via `writeSheet()` before finalize
- **THEN** peak memory for that phase SHALL be O(all sheets)

#### Scenario: Output phase holds one sheet at a time

- **WHEN** finalize emits the accumulated sheets
- **THEN** peak memory for the output phase SHALL be one sheet's XML plus the shared-strings and style accumulators

#### Scenario: Metadata parts are emitted after sheet XML

- **WHEN** finalize completes
- **THEN** `sharedStrings.xml`, `styles.xml`, and workbook metadata SHALL be emitted after all sheet XML

### Requirement: Streaming writer finalize targets

The streaming writer SHALL provide at least these output targets: `finalize()` returning a
`Buffer`, `finalizeToFile(path)` writing to a file on disk, and `finalizeToReadable()`
returning a JS `ReadableStream` of compressed chunk Buffers. Each target SHALL be
constant-memory in the output phase per the two-phase model; `finalize()` inherently
materializes the full output in RAM in its returned `Buffer`, and `finalizeToFile` /
`finalizeToReadable` buffer their input sheets in the handle.

#### Scenario: finalize returns an in-memory Buffer

- **WHEN** `finalize()` is called
- **THEN** it SHALL return the `.xlsx` as an in-memory Buffer without double-buffering sheet emits

#### Scenario: finalizeToFile writes to disk

- **WHEN** `finalizeToFile(path)` is called
- **THEN** it SHALL emit the `.xlsx` to that path via incremental zip file-entry flushing

#### Scenario: finalizeToReadable returns a chunk stream

- **WHEN** `finalizeToReadable()` is called
- **THEN** it SHALL emit the `.xlsx` as a JS `ReadableStream` of compressed chunk Buffers

### Requirement: Streaming reader bounds resource usage on untrusted input via caps

The streaming reader SHALL enforce a streaming size cap on every part it reads:
`xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, each `xl/worksheets/sheetN.xml`, and
`xl/sharedStrings.xml`. Both caps SHALL be documented as the streaming resource contract.

#### Scenario: Every read part is subject to the cap

- **WHEN** the reader reads `xl/workbook.xml`, the workbook rels, a sheet part, or `xl/sharedStrings.xml`
- **THEN** the streaming size cap SHALL be enforced on that part

### Requirement: Streaming reader caps actual decompressed bytes at MAX_ENTRY_BYTES

The cap SHALL bound the *actual* decompressed bytes read from a zip entry via a bounded
reader, not merely the size declared in the zip central directory, so that a part declaring
a small uncompressed size but decompressing to a much larger size cannot exhaust memory.
The cap SHALL be `MAX_ENTRY_BYTES` (16 MiB) per entry.

#### Scenario: Decompression-bomb entry is capped

- **WHEN** a zip entry declares a small uncompressed size but decompresses to far more than 16 MiB
- **THEN** the reader SHALL stop at the decompressed-byte cap rather than exhausting memory

#### Scenario: Entry within the cap reads normally

- **WHEN** a zip entry decompresses to at most 16 MiB
- **THEN** the reader SHALL read it fully

### Requirement: Streaming reader bounds per-sheet SAX events at MAX_EVENTS

The per-sheet SAX event count SHALL be bounded by `MAX_EVENTS` (5,000,000).

#### Scenario: Sheet exceeding the event cap is stopped

- **WHEN** a sheet's SAX event count exceeds `MAX_EVENTS`
- **THEN** the reader SHALL stop processing that sheet rather than continue unbounded

### Requirement: Streaming reader resolves shared formulas to translated formula text

The streaming XLSX reader SHALL resolve shared formulas (`<f t="shared">`) on the read path
so that a shared-formula *member* cell yields the same translated formula text as the
whole-workbook reader, not its cached `<v>` value.

#### Scenario: Member cell yields translated formula, not cached value

- **WHEN** the reader encounters a shared-formula member cell
- **THEN** it SHALL yield the translated formula text matching the whole-workbook reader, not the cached `<v>` value

### Requirement: Streaming reader shifts bare references in shared formulas

When resolving a shared-formula *member* cell, the streaming reader SHALL shift bare column
references (e.g. `A`) and bare row references (e.g. `5`) in the master formula text by the
member's offset, so that the resolved text matches what the whole-workbook (calamine) reader
produces. This extends shared-formula member resolution beyond `Cell` references (`A1`) and
`Cell` ranges (`A1:A3`).

#### Scenario: Bare column reference shifts by the member offset

- **WHEN** a shared-formula master text contains a bare column reference such as `A` (e.g. `=A+B`) and the member cell is shifted one column to the right
- **THEN** the streaming reader resolves the member to `=B+C`, matching the whole-workbook reader, not the unshifted `=A+B`

#### Scenario: Bare row reference shifts by the member offset

- **WHEN** a shared-formula master text contains a bare row reference such as `5` (e.g. `=A1*5`) and the member cell is shifted one row down
- **THEN** the streaming reader resolves the member to `=A2*6`, matching the whole-workbook reader, not the unshifted `=A1*5`

### Requirement: Streaming reader preserves non-reference tokens in shared formulas

The streaming reader SHALL NOT shift tokens that are not valid references — function names
such as `COLUMN` and `SUM`, and quoted strings — preserving them verbatim.

#### Scenario: Function names and quoted strings stay verbatim

- **WHEN** a shared-formula master text contains a function-name token (e.g. `COLUMN`, `SUM`) or a quoted string (e.g. `"A1"`)
- **THEN** the streaming reader copies those tokens verbatim and does not attempt to shift them, identical to the whole-workbook reader

### Requirement: Streaming reader collects shared formulas per sheet

The reader SHALL collect a per-sheet table of shared formulas, keyed by `si`, from the master
cells that stream by.

#### Scenario: Master cells populate the si-keyed table

- **WHEN** shared-formula master cells stream by for a sheet
- **THEN** the reader SHALL record them in a per-sheet table keyed by `si`

### Requirement: Streaming reader translates shared-formula member references

The reader SHALL translate each member's relative references by the offset between the member
cell position and the master cell position, preserving absolute (`$A$1`) and mixed (`A$1`)
references.

#### Scenario: Relative references shift by the member offset

- **WHEN** a shared-formula member is resolved relative to its master
- **THEN** its relative references SHALL be shifted by the offset between the member and master positions

#### Scenario: Absolute and mixed references are preserved

- **WHEN** a shared formula contains `$A$1` or `A$1` references
- **THEN** those references SHALL be preserved rather than shifted

### Requirement: Streaming reader keeps shared-formula tables bounded

The shared-formula table SHALL be bounded by the number of distinct shared formulas in the
sheet, and the reader SHALL NOT materialize the whole sheet, preserving the
`MAX_ENTRY_BYTES` / `MAX_EVENTS` streaming resource contract.

#### Scenario: Table size tracks distinct shared formulas

- **WHEN** a sheet contains many cells but few distinct `si` values
- **THEN** the shared-formula table SHALL be bounded by the number of distinct `si` values

#### Scenario: Shared-formula resolution does not materialize the sheet

- **WHEN** shared formulas are resolved on a large sheet
- **THEN** the reader SHALL NOT materialize the whole sheet