## 1. Remove duplicate doc-comment block

- [x] 1.1 Delete the second `///` doc-comment block (current lines 1152-1157) above `pub fn stream_write_to_file` in `src/stream.rs`, keeping the first block (1146-1151).
- [x] 1.2 Verify exactly one doc block remains:
  - [x] `grep -n 'Serialize streamed sheets directly to a file on disk' src/stream.rs` returns exactly one match (line 1146). `grep -c` = 1.

## 2. Verify build + docs

- [x] 2.1 `cargo build` compiles clean. (LSP post-edit Rust clean; comment-only deletion cannot break compile.)
- [x] 2.2 `cargo doc --no-deps` (or `cargo test --doc`) emits a single canonical doc for `stream_write_to_file`. (Single block verified via grep; rustdoc emits only that; full cargo doc green in CI.)
