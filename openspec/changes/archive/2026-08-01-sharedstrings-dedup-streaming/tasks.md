## 1. Test cross-sheet sharedStrings dedup

- [x] 1.1 Added `stream_write_dedups_shared_strings_across_sheets` in `src/stream.rs` (tests module): build two `StreamSheet`s sharing a string value, call `stream_write`, parse `xl/sharedStrings.xml` via `zip::ZipArchive` + `parse_shared_strings`, assert `strings.len() == 3` (one `<si>` per distinct) and `dup` appears once; round-trip `stream_read` asserts both sheets resolve the same shared value.
- [x] 1.2 Added `stream_write_dedups_shared_strings_within_sheet`: single sheet with two cells both `Text("dup")` → `sharedStrings.xml` contains one entry.

## 2. Align spec + verify

- [x] 2.1 Delta spec `specs/streaming-write-incremental/spec.md` authored in propose phase; pins the dedup guarantee with cross-sheet + within-sheet scenarios (Status unchanged — bridge buffer still deferred per ADR-005).
- [x] 2.2 `cargo test --lib stream_write_dedups_shared_strings` → 2 passed; `cargo clippy --lib -- -D warnings` clean.
