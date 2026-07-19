## ADDED Requirements

### Requirement: Streaming reader parses a workbook from a byte stream

The streaming reader SHALL parse a `.xlsx` from a readable byte stream and yield
worksheet rows incrementally, without materializing the entire workbook in
memory. It SHALL preserve cell values and styles for the streamed rows using
the same fidelity as the whole-workbook reader on the read path.

#### Scenario: Read rows incrementally from a stream

- **WHEN** the streaming reader is given a readable `.xlsx` byte stream and iterated
- **THEN** it yields each worksheet row in order with its cell values (number / string / boolean / date / formula) populated, and does not hold the full workbook in memory at once

#### Scenario: Streaming read preserves cell values

- **WHEN** a workbook written by the whole-workbook writer (or by Excel/ExcelJS) is read via the streaming reader
- **THEN** the read-back cell values equal the written values for every streamed row

### Requirement: Streaming writer emits a workbook to a byte stream

The streaming writer SHALL accept rows incrementally and emit a valid `.xlsx`
to a writable byte stream without buffering the whole workbook in memory.

#### Scenario: Write rows incrementally to a stream

- **WHEN** rows are added one-by-one to the streaming writer and the output stream is consumed
- **THEN** a valid `.xlsx` is produced containing exactly the added rows with their cell values preserved

#### Scenario: Streaming write round-trips through the streaming reader

- **WHEN** a workbook produced by the streaming writer is read back by the streaming reader
- **THEN** the read-back rows and cell values match what was written
