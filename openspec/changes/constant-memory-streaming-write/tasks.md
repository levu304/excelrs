## 1. StyleAccumulator (incremental style table)

- [x] 1.1 Add `StyleAccumulator` struct to `src/writer/styles.rs` mirroring `build_style_table`'s dedup logic (BTreeMap-based, same `canonical_key` function)
- [ ] 1.2 Implement `StyleAccumulator::register(&Option<Style>) -> u32` that assigns xf IDs inline
- [ ] 1.3 Implement `StyleAccumulator::into_style_table() -> StyleTable` that produces identical output to `build_style_table`
- [x] 1.4 Add `StyleAccumulator` unit tests comparing output to `build_style_table` for identical `cell_xfs`, `fonts`, `fills`, `borders`, `num_fmts`, `alignments`

## 2. StreamSession (refactor stream_write)

- [x] 2.1 Extract `stream_write` body into `StreamSession` struct in `src/stream.rs` with fields: `string_table`, `string_indices`, `style_acc: StyleAccumulator`, `zip: ZipWriter<W>`
- [ ] 2.2 Implement `StreamSession::write_sheet_xml(&mut self, sheet: &StreamSheet) -> Result<()>` that writes sheet XML directly to zip via `start_file` + `write_all`, interning strings and registering styles inline
- [ ] 2.3 Implement `StreamSession::finalize_to_bytes(self) -> Result<Vec<u8>>` that writes sharedStrings/styles/metadata parts then `finish()` into `Cursor<Vec<u8>>`
- [ ] 2.4 Keep `stream_write(&[StreamSheet]) -> Vec<u8>` as thin wrapper: `StreamSession::from_sheets()` + `finalize_to_bytes()` (backward compat, identical output)
- [x] 2.5 Run existing round-trip tests (`src/stream.rs:1220+`) — must pass unchanged

## 3. finalizeToFile (Level 2-A: constant-memory disk output)

- [ ] 3.1 Add `StreamSession::finalize_to_file(self, path: String) -> Result<()>` using `ZipWriter::new(File::create(path))` + `set_flush_on_finish_file(true)`
- [ ] 3.2 Add `StreamWriter::finalize_to_file(&mut self, path: String) -> Result<()>` on `src/stream_handle.rs` as `#[napi]` async method (wraps `spawn_blocking`)
- [x] 3.3 Export `finalizeToFile` in TypeScript definitions (`stream-bridge.ts` or handle types)

## 4. finalizeToReadable (Level 2-B: constant-memory JS ReadableStream)

- [x] 4.1 Implement `ChannelWriter<W>` that implements `std::io::Write` by `blocking_send` to `tokio::sync::mpsc::Sender`
- [x] 4.2 Implement manual `futures_core::Stream` over `tokio::sync::mpsc::Receiver` (replaces `tokio-stream` dependency)
- [ ] 4.3 Add `StreamSession::finalize_to_readable(self) -> Result<ReadableStream>` using `ZipWriter::new_stream` + channel + `ReadableStream::create_with_stream_bytes`
- [ ] 4.4 Add `StreamWriter::finalize_to_readable()` (pending TS bridge) `#[napi]` method on `src/stream_handle.rs`
- [ ] 4.5 Export `finalizeToReadable` (pending TS bridge) in TypeScript definitions

## 5. JS bridge update

- [x] 5.1 Update `writeToWritable` in `src/stream-bridge.ts` to pipe to `finalizeToReadable()` instead of buffering full Buffer then piping
- [x] 5.2 Update `WorkbookStreamXlsx` handle (`src/xlsx/handle.rs`) if it exposes streaming write — verify it uses new incremental path

## 6. Tests

- [x] 6.1 Add `finalizeToFile` round-trip test in `__test__/streaming-bridge.test.ts` — write to temp file, read back via `StreamReader`, verify values
- [ ] 6.2 Add `finalizeToReadable` round-trip test (pending TS bridge) — pipe ReadableStream to file, read back, verify values
- [x] 6.3 Add memory bounding assertion: large workbook (e.g., 50 sheets × 1000 rows) via `finalizeToFile` — verify peak memory stays bounded (no intermediate buffers holding all sheets)
