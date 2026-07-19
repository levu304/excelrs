# Proposal: v2.0.0 — Streaming XLSX + parity capstone

## Why

The v1.x drop-in ExcelJS-compat program is functionally complete: every targeted
v1.x+ parity-matrix row from the roadmap has shipped — tables (v1.1.0),
conditional formatting (v1.2.0), and worksheet-structure finish (v1.3.0). The
single remaining unshipped matrix area is **Streaming XLSX**, targeted for
v2.0.0. Today `excelrs` supports only whole-workbook in-memory read/write, so a
workbook too large to fit in memory cannot be processed at all — a hard gap
against ExcelJS's `stream.xlsx` reader/writer for data-heavy users.

v2.0.0 is the major-version capstone that (1) ships a SAX-based streaming I/O
architecture for large files, (2) formally declares the ExcelJS-4.4.0 v1.x
parity program complete, and (3) reserves — for the major bump — any breaking
API cleanup the streaming architecture requires. A MAJOR bump is the right
vehicle because streaming may necessitate breaking changes to the public
read/write surface.

## What Changes

- **Streaming XLSX reader (SAX-based)**: add a streaming reader that parses a
  `.xlsx` from a readable byte stream without materializing the whole workbook,
  exposing rows/cells incrementally (ExcelJS `stream.xlsx.read` parity).
- **Streaming XLSX writer (SAX-based)**: add a streaming writer that emits a
  `.xlsx` to a writable byte stream incrementally as rows are supplied, without
  holding the full workbook in memory (ExcelJS `stream.xlsx.write` parity).
- **Streaming capability spec**: new `streaming-xlsx` capability covering the
  streaming reader/writer contract and its round-trip guarantees.
- **Parity program complete**: advance the `streams` area to `shipped` in
  `exceljs-parity`; add the v2.0.0 release-recording scenario and a new
  requirement declaring the v1.x drop-in ExcelJS-4.4.0 parity program
  **complete** (all targeted v1.x matrix rows shipped), with the explicitly
  distant-deferred areas (charts, pivot tables, formula evaluation, themes-write,
  sheet state, tab color, default properties) recorded as intentionally out of
  the completed program.
- **BREAKING (reserved)**: v2.0.0 is a MAJOR bump. Any breaking change the
  streaming architecture demands (e.g., read/write entry-point or option-shape
  changes) SHALL be captured here; none are committed up front because the exact
  surface is gated on the Step-0 ExcelJS 4.4.0 streaming-API audit. If the audit
  proves no breaking change is required, v2.0.0 ships non-breaking and the bump
  signals the new streaming capability.

## Capabilities

### New Capabilities

- `streaming-xlsx`: Streaming XLSX I/O — SAX-based reader and writer that process
  large workbooks from/to byte streams without full in-memory materialization,
  with incremental row/cell access and round-trip guarantees.

### Modified Capabilities

- `exceljs-parity`: v2.0.0 advances the `streams` matrix area → `shipped` and
  declares the v1.x drop-in ExcelJS-4.4.0 parity program complete; adds the
  v2.0.0 release-recording scenario and a program-complete requirement.
- `release-verification`: add a release smoke scenario that exercises the
  streaming reader/writer round-trip on a large (memory-exceeding) workbook so a
  streaming regression fails the release before publish.

## Impact

- **Code**:
  - `src/xlsx/mod.rs` + `src/xlsx/handle.rs` — streaming handle / zip access over
    a byte stream (reuse existing zip plumbing).
  - `src/reader/xlsx.rs` — add a SAX/event-driven parse path (incremental
    `<sheetData>` row emission) alongside the existing whole-workbook path.
  - `src/writer/xlsx.rs` — add an incremental emit path that streams rows to the
    output stream.
  - `src/reader/mod.rs` + `src/writer/mod.rs` — streaming entry points.
  - `src/lib.rs` + `index.d.ts` — expose `stream.xlsx.read` /
    `stream.xlsx.write` (or idiomatic napi equivalents) on the public API.
- **APIs**: new napi surface for streaming read/write. Existing whole-workbook
  `read`/`write` behavior is unchanged (additive) unless the Step-0 audit forces
  a breaking change (reserved, see What Changes).
- **Dependencies**: consider a streaming-capable zip/zlib layer if the current
  in-memory zip crate cannot stream parts; otherwise reuse `quick-xml` (already a
  dep) for SAX. No new dependency expected unless the audit shows a gap.
- **Specs**: new `streaming-xlsx` spec; `exceljs-parity` advanced to complete;
  `release-verification` gains a streaming smoke scenario.
- **Parity domain**: closes the final unshipped matrix area (`streams`) and
  formally completes the v1.x drop-in ExcelJS-4.4.0 parity program; distant /
  deferred items remain out of scope per ROADMAP.
