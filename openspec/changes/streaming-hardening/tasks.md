## 1. Correctness — terminate the destination stream

- [x] 1.1 In `src/stream-bridge.ts` `writeToWritable`, replace `writable.write(buf, (err) => { ... })` with `writable.end(buf, (err) => err ? reject(err) : resolve())` so piped consumers receive `finish`.
- [x] 1.2 Confirm the public signature is unchanged and no other caller depends on the old never-ending behavior.

## 2. Safety — zip-bomb caps

- [x] 2.1 Add `MAX_ARCHIVE_ENTRIES: usize = 10_000` and `MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024` constants next to `MAX_ENTRY_BYTES` in `src/stream.rs`.
- [x] 2.2 In `StreamReader::constructor` (`src/stream_handle.rs`), after `ZipArchive::new` succeeds, reject if `archive.len() > MAX_ARCHIVE_ENTRIES` or `buffer.len() > MAX_ARCHIVE_BYTES` with a clear `ExcelrsError::Read`.
- [x] 2.3 Apply the same cap check in `stream_read` (`src/stream.rs`) after `ZipArchive::new`, for consistency on untrusted batch reads.

## 3. Read memory tightening

- [x] 3.1 Change `StreamParseState.data` from `Vec<u8>` to `Arc<[u8]>`; open the `ZipArchive<Cursor<Arc<[u8]>>>` **once** in the constructor and store it in `state`.
- [x] 3.2 In `next()`, use `archive.by_name(&path)` (no re-parse); drop the `data` field; wrap `shared`/`style_table` in `Arc` to avoid per-sheet full clones.
- [x] 3.3 Run `gitnexus impact` on `StreamReader::next` / `StreamReader::constructor` before and after to confirm no caller blast radius (public API unchanged).

## 4. Tests

- [x] 4.1 Add TS bridge tests (extend or new file) covering `read`/`write`/`readAsReadable`/`writeToWritable` round-trips and asserting `writeToWritable` fires `finish` on a `PassThrough`.
- [x] 4.2 Add a Rust unit test in `src/stream.rs` that builds a zip with > `MAX_ARCHIVE_ENTRIES` tiny entries and asserts the streaming reader rejects it.

## 5. Docs honesty

- [x] 5.1 Update `README.md` and the misleading comments in `src/stream-bridge.ts` to state the write path buffers all sheets (buffered, not streaming) while the read path is streaming.
- [x] 5.2 Fix `sheet.rowCount` doc examples (the field does not exist on `JsStreamSheet`; use `sheet.rows.length`).

## 6. Build & verify

- [x] 6.1 Rebuild via `napi build --pipe` (regenerates `index.js`/`index.d.ts` with glue) and run the TypeScript typecheck.
- [x] 6.2 Run `cargo test` (incl. new bomb test), `npm test`, and confirm CI is green on the feature branch.
