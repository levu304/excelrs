## Status: COMPILE — all 9 tasks done

## 1. Validate change artifacts

- [x] 1.1 `openspec validate align-streaming-specs --type change` → `{valid:true, issues:[]}` (note: `--change` is not a valid flag on `validate`; positional + `--type change` is the working form)
- [x] 1.2 `openspec status --change align-streaming-specs --json` → 4/4 artifacts (proposal, specs, design, tasks) `done`

## 2. Confirm implementation is already honest (no code edits needed)

- [x] 2.1 `index.d.ts` L196-213 — `finalizeToFile` and `finalizeToReadable` doc-comments already state "input is NOT constant-memory — sheets are accumulated in the writer handle ... O(all sheets)" + defer to `streaming-write-incremental` (unchanged)
- [x] 2.2 `src/stream_handle.rs` L418-423 / L442-447 — `finalize_to_file` / `finalize_to_readable` doc-comments already state input is NOT constant-memory + defer to `streaming-write-incremental` (unchanged)
- [x] 2.3 `src/stream-bridge.ts` L84-86 — `writeToWritable` doc-comment already states "Input sheets are still buffered in the writer handle first, so the write path is not fully constant-memory" + defers to `streaming-write-incremental` (unchanged)

## 3. Archive (merge MODIFIED deltas into main specs)

- [x] 3.1 `openspec archive align-streaming-specs -y --json` → `{archivedAs: 2026-08-01-align-streaming-specs, specsUpdated: true, modified: 3}`
  - (first archive attempt failed: delta requirement header `Streaming writer finalize directly to file path` did not match main spec header `Streaming writer can finalize directly to a file path`; after syncing headers + the merge tool's scenario-name superset check, second attempt succeeded)
- [x] 3.2 `git diff --stat` + grep on the 3 main specs → contradictory phrase `at no point shall the process buffer more than one sheet's XML` is GONE; new two-phase wording `Input is NOT constant-memory` / `O(all sheets)` + cross-references to `streaming-write-incremental` + `docs/adr/005-streaming-write-buffering.md` PRESENT in all 3

## 4. Regression guard (docs-only: no source edits)

- [x] 4.1 `npx vitest run __test__/streaming-bridge.test.ts` → 12 passed (1 file) in 2.47s (identical to pre-change; no source was edited)
- [x] 4.2 `openspec status` confirmed 4/4 `done` before archive; post-archive CLI confirms `specsUpdated:true, modified:3` — change archived, active change no longer listed (expected)
