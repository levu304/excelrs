# streaming-node-bridge Specification

## Purpose

The Node.js streaming bridge for XLSX: a hand-written `src/stream-bridge.ts`
wrapper over the `StreamReader`/`StreamWriter` napi classes that exposes
constant-memory XLSX read/write to Node via `AsyncIterable`, `Readable`, and
`Writable`. (Created by archiving change streaming-node-bridge.)

## Requirements

### Requirement: Streaming reader yields an async iterable of sheets

The streaming reader SHALL expose `wb.stream.xlsx.read(input)` returning an
`AsyncIterable<StreamSheet>` that yields one worksheet at a time, materializing
only the current sheet's rows in memory (not the entire workbook). Each yielded
`StreamSheet` SHALL carry the same cell values (number / string / boolean /
formula) as the v2.0.0 `StreamSheet` shape.

#### Scenario: Read yields sheets one at a time

- **WHEN** `read` is given a seekable source (a `Buffer` or a file path) and iterated with `for await`
- **THEN** it yields each worksheet as a `StreamSheet` in document order, and at no point holds all sheets in memory simultaneously

#### Scenario: Read is also obtainable as a Node Readable

- **WHEN** the async iterable returned by `read` is wrapped with `Readable.from(iter)`
- **THEN** a valid Node `Readable` is produced that emits the same `StreamSheet` objects

### Requirement: Streaming writer accepts an async iterable of sheets

The streaming writer SHALL accept an `AsyncIterable<StreamSheet>` as input and
emit a valid `.xlsx` without requiring the full sheet array to be buffered in
memory. Output SHALL stream to a `Buffer`, a Node `Writable`, or a file path.

#### Scenario: Write from an async iterable

- **WHEN** `write` is given an `AsyncIterable<StreamSheet>` (produced incrementally by the caller)
- **THEN** a valid `.xlsx` is produced containing exactly the supplied sheets with cell values preserved, and the writer does not buffer the entire workbook

#### Scenario: Write streams to a Writable / file path

- **WHEN** `write` targets a Node `Writable` or a file path
- **THEN** sheet entries are emitted incrementally to that sink as they are supplied

### Requirement: Streaming bridge preserves values and empty-cell distinction

The bridge SHALL preserve cell values round-trip and SHALL distinguish an empty
cell (no value) from a cell holding an empty string `""`, identical to the
`streaming-xlsx` contract.

#### Scenario: Empty cell round-trips as empty

- **WHEN** an empty JS cell (`{ value: null }`) is written via the streaming writer and read back via the streaming reader
- **THEN** the read-back cell is an empty cell, not a text cell holding `""`

### Requirement: Streaming bridge is non-breaking and values-only

The bridge SHALL be additive: the existing `read(buffer): Promise<StreamSheet[]>`
and `write(sheets): Promise<Buffer>` forms SHALL remain available. Per-cell
styles SHALL stay on the in-memory `xlsx` path; the streaming bridge carries cell
values only.

#### Scenario: Existing Promise forms still work

- **WHEN** a caller uses the v2.0.0 `read(buffer): Promise<StreamSheet[]>` / `write(sheets): Promise<Buffer>` forms
- **THEN** behavior is unchanged and no breaking change is required

### Requirement: Streaming bridge propagates errors cleanly mid-stream

The bridge SHALL surface hostile-input / resource-cap errors (the `streaming-xlsx`
`MAX_ENTRY_BYTES` / `MAX_EVENTS` contract) as a rejected iteration / write,
aborting without leaking partial parse state.

#### Scenario: Mid-stream cap error aborts the iterator

- **WHEN** a streamed part exceeds the size/event cap while iterating or writing
- **THEN** the current `next()` / `write_sheet()` rejects with a clear error and the iterator/writer is left in a closed, consistent state
