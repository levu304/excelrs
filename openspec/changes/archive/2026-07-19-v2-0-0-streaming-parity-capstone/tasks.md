## 1. Audit & API contract

- [x] 1.1 Run the Step-0 ExcelJS 4.4.0 streaming-API audit (`npm i exceljs@4.4.0`): capture `stream.xlsx.read` / `stream.xlsx.write` surface, emitted OOXML, and event/option shapes.
- [x] 1.2 Decide the napi streaming surface (Node `Readable`/`Writable` vs `AsyncIterable`) and document the chosen bridge in `design.md` (D4).
- [x] 1.3 Produce the breaking-change inventory (if any) for the 2.0.0 MAJOR bump; if empty, record v2.0.0 as planned non-breaking in `design.md` Open Questions.

## 2. Streaming reader

- [x] 2.1 Add a per-entry streaming reader over the `zip` v7 crate that opens `xl/sharedStrings.xml`, `xl/styles.xml`, and each `xl/worksheets/sheetN.xml` as streams (reuse existing zip plumbing in `src/xlsx/`).
- [x] 2.2 Read shared-strings and styles parts once up front into maps (D3); reuse existing shared-string / style resolution from the whole-workbook reader.
- [x] 2.3 Add a SAX parse path in `src/reader/xlsx.rs` using `quick-xml` that emits `Row`/`Cell` objects incrementally from `<sheetData>` (same `Cell`/`Row`/`Style` model as the in-memory reader).
- [x] 2.4 Expose a streaming read entry point in `src/reader/mod.rs` yielding rows from a readable byte stream without holding the full workbook in memory.
- [x] 2.5 Bridge the streaming reader across the FFI boundary via `napi` `async` + `tokio` (D4). _Note: v2.0.0 bridges via `Buffer` in / `StreamSheet[]` out, not a live Node stream/AsyncIterable (deferred — see design.md Open Questions)._

## 3. Streaming writer

- [x] 3.1 Add a sequential `ZipWriter` append path in `src/xlsx/` that emits `xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, `xl/styles.xml`, `xl/sharedStrings.xml`, and each sheet part incrementally.
- [x] 3.2 Add an incremental `<sheetData>` emit path in `src/writer/xlsx.rs` that streams rows to the output stream as they are supplied.
- [x] 3.3 Expose a streaming write entry point in `src/writer/mod.rs` accepting rows one-by-one and writing to a writable byte stream.
- [x] 3.4 Bridge the streaming writer across the FFI boundary via `napi` `async` + `tokio`. _Note: v2.0.0 bridges via `StreamSheet[]` in / `Buffer` out (deferred live Node stream — see design.md Open Questions)._

## 4. Public API & types

- [x] 4.1 Expose the streaming surface (`stream.xlsx.read` / `stream.xlsx.write`) in `src/lib.rs` (via `src/stream_handle.rs`).
- [x] 4.2 Add the matching TypeScript declarations to `index.d.ts` and verify with `tsc --noEmit`.
- [x] 4.3 Add unit/round-trip tests asserting streamed cell values survive (Rust tests in `src/stream.rs`; streaming smoke added to release.yml — task 6.1).

## 5. Parity program complete

- [x] 5.1 Update `openspec/specs/exceljs-parity/spec.md` / `ROADMAP.md` to mark `workbook IO (streams)` → `shipped` and record the v2.0.0 scenario.
- [x] 5.2 Update `ROADMAP.md` to declare the v1.x drop-in ExcelJS-4.4.0 parity program complete, listing charts / pivot tables / formula evaluation / themes-write / sheet state / tab color / default properties as out of scope.
- [ ] 5.3 Archive this change's `exceljs-parity` delta spec into the main spec on merge. _(Merge/release-time action via `openspec archive`.)_

## 6. Release verification

- [x] 6.1 Add a streaming round-trip smoke scenario to `.github/workflows/release.yml` (and `scripts/streaming-smoke.cjs`) exercising a workbook through the streaming writer then reader (covers `release-verification` delta).
- [x] 6.2 Confirm the existing in-memory smoke assertions still pass alongside the new streaming ones.

## 7. Version bump & changelog

- [x] 7.1 Bump `package.json` from `1.3.0` to `2.0.0`.
- [x] 7.2 Add the v2.0.0 entry to `CHANGELOG.md` (streaming XLSX + parity-program-complete; note any reserved breaking change).
- [ ] 7.3 Tag-driven release: `git tag -a v2.0.0` per the documented release process. _(Release-time action.)_

### Implementation notes

- **FFI shape (v2.0.0):** `workbook.stream.xlsx.read(buffer: Buffer): Promise<StreamSheet[]>` and `write(sheets: StreamSheet[]): Promise<Buffer>`. The Rust core streams row-by-row (per-entry zip + SAX) and avoids building the full `Workbook` model; the FFI collects sheet objects into a JS array. Constant-memory Node `Readable`/`Writable` / `AsyncIterable` bridging is a **deferred follow-up** (the Rust core already streams; only the FFI collection is array-based in v2.0.0).
- **Non-breaking:** streaming is purely additive (new `stream` namespace); all 1.x APIs are unchanged, so no MAJOR-bump breaking change was required.
- **Styles:** per-cell styles are surfaced via the in-memory `xlsx` path; the streaming path carries cell _values_ (number / string / boolean / formula).
- **Streaming tests:** Rust unit/round-trip tests in `src/stream.rs` cover write→read, stream-write→in-memory-read, and in-memory-write→stream-read. The CI smoke (`scripts/streaming-smoke.cjs`) covers the JS FFI path.
