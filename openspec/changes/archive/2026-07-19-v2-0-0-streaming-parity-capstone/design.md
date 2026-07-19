# Design: v2.0.0 — Streaming XLSX + parity capstone

## Context

`excelrs` is a napi-rs native addon porting the ExcelJS API. Every targeted
v1.x parity-matrix row has now shipped (tables v1.1.0, conditional formatting
v1.2.0, worksheet-structure finish v1.3.0). The ROADMAP pins the last area —
**Streaming XLSX** — and the v1.x drop-in ExcelJS-4.4.0 parity program
completion to **v2.0.0**.

Today both read and write materialize the whole workbook in memory
(`src/reader/xlsx.rs`, `src/writer/xlsx.rs`, `src/xlsx/`). A workbook larger
than available RAM cannot be processed at all. ExcelJS closes this gap with
`stream.xlsx.read` / `stream.xlsx.write`, which stream rows to/from byte
streams. v2.0.0 delivers that and uses the MAJOR bump to formally complete the
parity program and absorb any breaking cleanup the streaming surface requires.

The dependency tree already supports streaming with **zero new crates**:
`zip = "7"` (per-entry streaming reads via `ZipFile` and sequential
`ZipWriter` appends), `quick-xml = "0.36"` (SAX `Reader` for event-driven XML),
and `napi` (`async` feature) + `tokio` (already in the tree) for streaming
across the JS boundary.

## Goals / Non-Goals

**Goals:**

- Ship a SAX-based streaming reader that yields rows/cells incrementally from a
  readable `.xlsx` byte stream, without holding the full workbook in memory.
- Ship a streaming writer that emits a valid `.xlsx` to a writable byte stream as
  rows are supplied, without buffering the whole workbook.
- Match ExcelJS `stream.xlsx` fidelity for **cell values + styles + core
  structure** on the streaming path.
- Advance the `streams` area → `shipped` and declare the v1.x drop-in
  ExcelJS-4.4.0 parity program complete.
- Reserve (for the MAJOR bump) any breaking change the streaming surface needs,
  gated on the Step-0 ExcelJS 4.4.0 streaming-API audit.

**Non-Goals:**

- Streaming of charts, pivot tables, or formula evaluation (all distant-deferred
  per ROADMAP).
- Per-row streaming of rich parts (comments / images / drawings / tables) in the
  first streaming release — these remain whole-workbook features. (Matches
  ExcelJS `stream.xlsx` limitations.)
- Removing or changing the existing in-memory `read`/`write` behavior.
- The post-v2.0.0 triage items (themes-write, sheet state, tab color, default
  properties).

## Decisions

**D1 — Reuse the `zip` crate's per-entry streaming; add no zip dependency.**
The `zip` v7 crate reads each entry as a stream (no full extraction) and
`ZipWriter` appends entries sequentially, so a workbook can be read/written
part-by-part. Switching to another zip crate would add a dependency for no
gain. *Alternative considered:* a fully async zip crate — rejected; it would
duplicate the existing zip layer and is unnecessary for part-at-a-time I/O.

**D2 — SAX parse `<sheetData>` with `quick-xml` (already a dep).**
`quick-xml` 0.36's event `Reader` drives incremental row emission, reusing the
shared-string and style resolution already built for the whole-workbook reader.
*Alternative considered:* a new pull-parser crate — rejected; `quick-xml` is
installed and proven in the codebase.

**D3 — Emit shared-strings + styles parts once up front, then stream sheets.**
Shared strings (`xl/sharedStrings.xml`) and styles (`xl/styles.xml`) are read
into maps before sheets are streamed. This bounds memory to *data size*, not
part count, and lets each streamed row resolve its string/style via the same
lookups the in-memory reader uses. *Alternative considered:* lazily paging
shared strings — rejected as premature complexity for the common case where
these parts are far smaller than cell data.

**D4 — Expose streaming via napi async (post-audit surface).** The Step-0 ExcelJS 4.4.0 audit confirmed `workbook.stream.xlsx.read(stream)` / `stream.xlsx.write(workbook)` as the streaming surface. We mirror it as `workbook.stream.xlsx.read(buffer): Promise<StreamSheet[]>` and `workbook.stream.xlsx.write(sheets: StreamSheet[]): Promise<Buffer>` — async napi methods bridging Node via `Buffer` in / `Buffer` out. The Rust core parses/serializes incrementally (per-entry zip + SAX) without building a full `Workbook` model, so per-cell object overhead is avoided; the FFI collects sheet objects into a JS array in v2.0.0. Constant-memory Node `Readable`/`Writable` / `AsyncIterable` bridging is a deferred follow-up (see Open Questions).

**D5 — Reuse the existing `Cell` / `Row` / `Style` model on the streaming path.**
The streaming reader produces the same `Cell`/`Row` objects as the in-memory
reader; only *when* rows are emitted (incrementally) and *what* is retained
(per-sheet cell values, not the full `Workbook`) differs. This keeps fidelity
and test parity with the existing path.

## Risks / Trade-offs

- **[Shared-strings + styles still loaded up front]** → memory is bounded by
  data, not parts, but those maps exist. *Mitigation:* document the bound; it is
  the same model the in-memory reader uses, just without the sheet cell bulk.
- **[Reduced fidelity on streaming path]** → rich parts (comments/images/
  drawings/tables) are out of scope for v1 of streaming. *Mitigation:* scoped
  explicitly as Non-Goals; the in-memory path remains the full-fidelity route.
- **[Breaking change surface]** → the new `stream` namespace or option shape
  may break callers. *Mitigation:* reserved under the MAJOR bump; exact surface
  fixed by the Step-0 audit and recorded in CHANGELOG before implementation.
- **[zip crate cannot truly stream a needed part]** → *Mitigation:* zip v7
  per-entry streaming covers it; the audit confirms before design freeze.

## Migration Plan

- The streaming surface is additive (new `stream.xlsx.read` / `stream.xlsx.write`
  namespace); existing `read`/`write` are untouched, so default behavior needs
  no migration.
- If the Step-0 audit forces a breaking change, it ships only under the 2.0.0
  MAJOR bump with CHANGELOG migration notes; the prior major (1.x) API stays
  intact on the `1.x` line.
- Rollback: gate the streaming feature behind its own namespace; no change to
  the in-memory read/write code path, so a revert is localized.

## Open Questions

- **napi surface shape** — *Resolved.* Buffer-in/buffer-out async methods (`read`/`write`) returning Promises; cell values cross the FFI, styles stay on the in-memory path. Constant-memory Node `Readable`/`Writable` / `AsyncIterable` bridging is **deferred** to a post-v2.0.0 follow-up; the Rust core already streams row-by-row.
- **Per-sheet vs workbook-wide streaming** — *Resolved.* Workbook-wide: `read` returns all sheets, `write` accepts all sheets, mirroring ExcelJS `stream.xlsx` whole-workbook orientation.
- **Breaking-change inventory** — *Resolved: none.* v2.0.0 is non-breaking — streaming is purely additive via the new `stream` namespace; existing `read`/`write` and all 1.x APIs are unchanged, so no MAJOR-bump breaking change was required.
- **Shared formulas (`t="shared"`)** — *Known limitation.* A shared-formula *member* cell (`<c r="A1"><f t="shared" si="0"/><v>3</v></c>`) carries no inline formula text; the string lives only in the master cell. The streaming reader does **not** resolve `si` against the master, so member cells round-trip as their cached value, not `Formula`. Inline (non-shared) formulas round-trip correctly. Resolving shared formulas requires cross-cell master lookup — deferred; document as a v2.0.0 limitation.
- **Size / event caps** — workbook.xml, its `.rels`, each sheet, and sharedStrings are read whole into a `String` and bounded by `MAX_ENTRY_BYTES` (16 MB, errors rather than silently truncating). `MAX_EVENTS` (5M) is a safety valve that errors instead of silently truncating. A single sheet larger than 16 MB (or 5M XML events) exceeds the cap; lowering/clarifying these limits is a follow-up if true single-row streaming is needed.
