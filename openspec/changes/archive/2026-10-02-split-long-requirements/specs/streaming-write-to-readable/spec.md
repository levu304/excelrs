## REMOVED Requirements

### Requirement: Streaming writer can produce a JS ReadableStream of chunks

**Reason**: Bundled the `finalizeToReadable` API contract, the output-phase backpressure guarantee, and a full restatement of the deferred input-phase disclaimer into one over-long requirement.

**Migration**: Replaced by "Streaming writer can produce a JS ReadableStream of chunks" and "finalizeToReadable streams output with bounded backpressure", with the deferred-scope disclaimer replaced by a cross-reference to the `streaming-xlsx` two-phase model.

## ADDED Requirements

### Requirement: Streaming writer exposes finalizeToReadable returning a ReadableStream

The streaming writer SHALL provide a `finalizeToReadable()` method that returns a JS
`ReadableStream` yielding compressed zip chunk `Buffer`s. Its input- and output-phase memory
behavior SHALL be as stated by the two-phase model in
`openspec/specs/streaming-xlsx/spec.md`. True incremental `writeSheet()` remains deferred per
`openspec/specs/streaming-write-incremental/spec.md` and
`docs/adr/005-streaming-write-buffering.md`.

#### Scenario: finalizeToReadable returns a chunk stream

- **WHEN** `finalizeToReadable()` is called
- **THEN** it SHALL return a JS `ReadableStream` yielding compressed zip chunk `Buffer`s

#### Scenario: finalizeToReadable buffers input sheets in the handle

- **WHEN** sheets are accumulated via `writeSheet()` before `finalizeToReadable`
- **THEN** peak process memory SHALL be O(all sheets), as stated by the `streaming-xlsx` two-phase model

### Requirement: finalizeToReadable streams output with bounded backpressure

During the output-emission phase, the writer SHALL write each zip file entry as its sheet XML
is produced and pipe the compressed bytes through the stream via a bounded mpsc channel
(cap 16), so that phase never holds more than one sheet's XML worth of cell data plus the
shared-strings and style accumulators in memory at a time.

#### Scenario: Output phase holds one sheet at a time

- **WHEN** `finalizeToReadable` emits sheets as a `ReadableStream`
- **THEN** at most one sheet's XML plus the shared-strings and style accumulators SHALL be held in memory at a time

#### Scenario: Backpressure is bounded at cap 16

- **WHEN** the consumer reads the `ReadableStream` slowly
- **THEN** the writer SHALL apply backpressure through a bounded channel of cap 16