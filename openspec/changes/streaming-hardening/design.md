## Context

The `streaming-node-bridge` change (PR #34) shipped a pull-based `StreamReader`
(async iterator), a buffered `StreamWriter`, and a hand-written TS bridge
(`src/stream-bridge.ts`) exposing `read`/`write`/`readAsReadable`/`writeToWritable`.
A code review of that PR confirmed three High-impact defects and two secondary
gaps (see `proposal.md` / Why).

Current read flow (`StreamReader`):

- ctor: `buffer.to_vec()` → `state.data: Vec<u8>` (held for the reader's lifetime),
  then `ZipArchive::new` once to parse targets/shared/styles.
- `next()`: clones `state.data` (full file) and re-runs `ZipArchive::new` **per
  sheet**, then reads one entry. So peak memory ≈ 2× file + N× central-dir parse,
  and there is no cap on entry count / total size at `ZipArchive::new`.

Current write flow (bridge `write` → `StreamWriter`):

- `write` collects every sheet into `StreamWriter.sheets: Vec<StreamSheet>` and
  `finalize()` builds the whole archive in RAM → one `Buffer`. Not streaming.
- `writeToWritable` calls `writable.write(buf, cb)` but never `writable.end()` →
  destination hangs.

There is a per-entry `take(MAX_ENTRY_BYTES)` (16 MB) guard in `stream.rs`, but it
only applies *after* the central directory is already parsed — so it does not stop
a zip-bomb delivered as many tiny entries.

## Goals / Non-Goals

**Goals:**

- Make `writeToWritable` terminate destination streams (`end()`).
- Reject zip-bomb inputs (entry-count + total-size caps) on untrusted `.xlsx`.
- Remove the per-sheet full clone and central-directory re-parse in `StreamReader`
  so peak memory stays bounded by one sheet.
- Add regression tests for the TS bridge functions and for the entry-count bomb.
- Make README/comments honest about the (buffered) write path.

**Non-Goals:**

- **True constant-memory *write* to a `Writable`** (streaming zip encoder that
  avoids `Seek` + inline-string cells). The `zip` v7 `ZipWriter` requires
  `Write + Seek`; feeding a non-seekable Node `Writable` needs a streaming encoder
  and a larger design. Deferred to a separate change.
- Per-cell style streaming on the stream path (already deferred in `streaming-node-bridge` design D4).
- Changing any public napi signatures.

## Decisions

- **D1 — `writeToWritable` end():** replace `writable.write(buf, cb)` with
  `writable.end(buf, (err) => err ? reject(err) : resolve())`. One-line correctness fix;
  public signature unchanged.

- **D2 — zip-bomb caps (constants):** add `MAX_ARCHIVE_ENTRIES: usize = 10_000`
  and `MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024` (256 MB) near `MAX_ENTRY_BYTES`
  in `stream.rs`. In `StreamReader::constructor` and `stream_read`, after
  `ZipArchive::new` succeeds, reject if `archive.len() > MAX_ARCHIVE_ENTRIES` or
  `buffer.len() as u64 > MAX_ARCHIVE_BYTES` with a clear `ExcelrsError::Read`.
  Rationale: bounds the central-directory parse (the real attack surface) before
  any content is read. 10k entries / 256 MB covers legitimate workbooks with large
  margins while stopping bombs.

- **D3 — read memory tightening:** change `StreamParseState.data` from `Vec<u8>` to
  `Arc<[u8]>` (clone = 8 bytes). Open the `ZipArchive` **once** in the ctor as
  `ZipArchive<Cursor<Arc<[u8]>>>`, store it in `state`, and in `next()` call
  `archive.by_name(&path)` (no re-parse). Wrap `shared: Vec<String>` and
  `style_table: Option<StyleTableRead>` in `Arc` so per-sheet access is a cheap
  clone, not a full copy. The `data` field can be dropped from `state` since the
  archive now owns the bytes via its `Cursor<Arc<[u8]>>`. Public API unchanged.

- **D4 — tests:** add a TS bridge test file (or extend `__test__/streaming-bridge.test.ts`)
  covering `read`/`write`/`readAsReadable`/`writeToWritable` round-trips and that
  `writeToWritable` actually ends a `PassThrough` (asserts `finish` fires). Add a
  Rust unit test in `src/stream.rs` that builds a zip with > `MAX_ARCHIVE_ENTRIES`
  tiny entries and asserts `StreamReader` (or `stream_read`) rejects it.

- **D5 — doc honesty:** update `README.md` and the misleading comments in
  `src/stream-bridge.ts` (`write`/`writeToWritable`) to state the write path buffers
  all sheets in memory (buffered, not streaming) while the read path is streaming.
  Fix the `sheet.rowCount` doc examples (the field does not exist on `JsStreamSheet` —
  only `name` + `rows`).

## Risks / Trade-offs

- **Cap values are heuristics.** 10k entries / 256 MB may reject a legitimate but
  extreme workbook. Chosen generously; can be raised if a real use case needs more.
  Not configurable in v1 (keeps the surface minimal).
- **`Arc` adds minor indirection** in `next()`, but eliminates a full-file copy per
  sheet — a clear win for large multi-sheet files.
- **`stream_read` (batch read) also gets the cap** for consistency, even though it
  opens the archive only once; this is a strict safety improvement with no behavior
  change for valid inputs.
- **Write path stays buffered.** This is deliberate (D2 Non-Goal). Docs must not
  overclaim; the "constant-memory" promise applies to the read path.
