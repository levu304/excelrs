# streaming-write-incremental Specification

## Status

> ⚠️ **Deferred (not yet implemented).** The v2.2.0 streaming writer
> (`src/stream_handle.rs` `StreamWriter`) does **not** satisfy this spec: its
> `writeSheet()` accumulates every sheet into a `sheets: Vec<StreamSheet>`
> buffer, and each `finalize*` clones that buffer before writing. Peak write
> memory is therefore **O(all sheets)**, and the "constant-memory" claims on
> `finalizeToFile`/`finalizeToReadable`/`writeToWritable` are inaccurate for the
> write path (they hold only for *output* emission).
>
> This spec remains the **target** for a future change (true incremental
> `writeSheet`, with the handle owning an open `ZipWriter` across FFI calls).
> Decision rationale: see `docs/adr/005-streaming-write-buffering.md`.
> See also issue #25.1 and the reverted prior attempt (commit `c19a4fc`).

## Purpose

Defines the incremental sheet-writing behavior that makes the streaming XLSX
writer truly constant-memory for its internal intermediate buffers.

## Requirements

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

#### Scenario: Incremental string interning deduplicates within a sheet

- **WHEN** the same string value appears in two cells of a single sheet
- **THEN** `sharedStrings.xml` contains that string exactly once and both cells reference the same `<v>` index

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

### Requirement: finalizeToReadable is cancelable + self-cleaning

`finalizeToReadable`'s detached zip-writer worker MUST terminate promptly (≤2 s,
not the ~55–60s GC window) and release the `ZipWriter`, `StreamSession`, and
bounded mpsc channel whenever the consumer abandons the `ReadableStream`,
whether by **explicit cancel** or by **drop-without-release**.

A *live* consumer that keeps draining MUST receive all chunks exactly once in
order, with cap-16 backpressure preserved (byte-identical emission to prior
behavior); cancellation/abandon MUST NOT surface as a write error to a live
consumer and MUST NOT corrupt the zip for a live reader.

The JS consumer bridge (`writeToWritable`) MUST release/abandon the stream on its
own early exit (`readable.cancel()` + `reader.releaseLock()` in a `finally`) —
this change does NOT implement true incremental `writeSheet` (ADR-005 Path A,
out of scope); it hardens output-phase teardown only.

#### Scenario: Explicit cancel terminates the worker promptly

- **WHEN** `finalizeToReadable` is mid-emit and the consumer calls
  `readable.cancel()` before the zip writer finishes
- **THEN** the detached worker thread exits within a bounded window (≤2 s),
  the held `ZipWriter` + `StreamSession` are dropped, and the channel reports
  EOF/`Closed` (no ~55–60s GC wait)

#### Scenario: Live consumer still gets full output plus backpressure

- **WHEN** a consumer reads the full `ReadableStream` normally (does not cancel)
- **THEN** all zip chunks are delivered exactly once in order and consumer-driven
  backpressure is preserved (cap-16 respected, no spin, live path byte-identical
  to prior behavior)

#### Scenario: Abandon-without-cancel still terminates (Rust backstop)

- **WHEN** a caller drops/cancels at the JS level but napi defers dropping the
  underlying Rust `Stream` (B2 uncertainty)
- **THEN** the Rust worker MUST still self-terminate (does not park forever),
  because `ChannelWriter::write` uses non-parking `tokio` `try_send` with
  cap-16 backoff and returns `Err` on `Closed` (consumer gone) — the `Closed`
  variant / `is_closed()` is the authoritative guard (no `Arc<AtomicBool>` per
  design).
