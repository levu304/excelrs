# sharedstrings-dedup-streaming — Design

## Context

`StreamSession` (src/stream.rs:84) holds `string_indices: HashMap<String, u32>`,
initialized empty in `StreamSession::new` and reused for every `write_sheet_xml`
call across the `stream_write` / `stream_write_to_file` loops (stream.rs:1138,
1140, 1157, 1159). Text cell values enter via `entry(s).or_insert_with`:

```rust
let idx = *self.string_indices.entry(s.clone()).or_insert_with(|| {
    let i = self.string_table.len() as u32;
    self.string_table.push(s.clone());
    i
});
```

`entry().or_insert_with` assigns the index on first insertion and returns it
unchanged thereafter, so the dedup is **structural** and **cross-sheet**.
`finalize` flushes `self.string_table` via `write_shared_strings`; no
sheet-level state is reset between `write_sheet_xml` calls (only
`current_sheet_index` advances). The deferred
`streaming-write-incremental` spec's interning scenario is therefore already
implemented — only the JS-bridge `Vec<StreamSheet>` buffer (stream_handle.rs)
remains O(sheets) and deferred.

## Decisions

- **Scope = verify + guarantee, not re-arch.** The interner already dedups
  session-wide; the defect is that it is untested. Moving the interner to the
  JS-bridge layer is rejected — it re-opens the per-session buffer problem
  ADR-005 records for reverted #34 (`c19a4fc`) and PR #49 Path A.
- **Test surface:** `stream_write` (returns `Vec<u8>`) → parse with
  `zip::ZipArchive` → read `xl/sharedStrings.xml` (reuse `parse_shared_strings`,
  stream.rs:508) + one sheet XML → assert one `<si>` per distinct string and an
  identical `<v>` index for a shared string across sheets.
- **Memory:** `string_indices`/`string_table` are `O(distinct strings)` —
  unchanged, constant-memory-friendly, consistent with ADR-005.

## Risks / Trade-offs

- The `count` attribute (total occurrences) is not pinned — only `uniqueCount ==
  distinct` and identical cross-sheet index. Loose assertion keeps the test
  robust to future occurrence accounting (see proposal Open Questions).
