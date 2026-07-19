## Why

v2.0.0 shipped the streaming XLSX reader/writer (PR #24). A post-merge qa-expert
audit (issue #26) verified five residual risks (A1–A5) in the streaming core, and
issue #25 tracked deferred limitations. This change hardens the streaming path
before building larger features (the Node stream bridge and shared-formula
resolution, deferred to later changes): it fixes a sheet-resolution correctness
bug, removes redundant zip I/O, closes a hostile-input OOM vector, preserves
empty-cell fidelity, and makes the streaming size caps correct. A4 (formula-capture
reset) was already merged in v2.0.0 pre-merge hardening and needs only verification
plus a regression test.

## What Changes

- **A1 — Sheet file resolution.** `sheet_number_from_target` does a digit-greedy
  filename parse (`sheet_v2.xml` → `2` → opens the wrong file; also breaks when a
  filename's number disagrees with document order). Resolve the sheet file directly
  from the `workbook.xml.rels` target path instead of re-deriving `sheetN.xml` from
  extracted digits. Removes the fragile parse entirely.
- **A2 — Redundant zip I/O.** `stream_read` re-opens the `ZipArchive` 3+ times per
  call (once in `stream_read`, again in `parse_workbook_sheet_targets`, again in
  `parse_shared_strings`, and once more inside style parsing). Open it once and pass
  `&mut ZipArchive` to the helpers.
- **A3 — Hostile-input OOM guard.** The current cap checks `entry.size()`, the
  *declared* uncompressed size in the zip central directory. A zip can declare 1 KB
  and decompress to 1 GB, bypassing the guard and OOMing on `read_to_string`. Bound
  the *actual* bytes read with `entry.take(MAX_ENTRY_BYTES)` (the pattern already used
  correctly in `reader/styles.rs`).
- **A4 — Formula-capture reset.** Already fixed in v2.0.0 (spec requirement merged in
  `09c6164`); the `in_f` flag is reset at the cell boundary (`stream.rs` `</c>` handler).
  No code change — add a regression test that feeds malformed XML missing `</f>`.
- **A5 — Empty-cell fidelity.** `from_js_value` maps an empty JS cell (`{value: null}`
  / `{}`) to `StreamValue::Text("")`, collapsing the empty-vs-`""` distinction on
  round-trip (ExcelJS parity gap). Add an `Empty` variant so an empty cell round-trips
  as empty.
- **#25.3 — Cap tuning (coupled with A3).** Apply the real-byte bound from A3
  consistently across every streamed entry (workbook.xml, its rels, each sheet,
  sharedStrings.xml) and document the `MAX_ENTRY_BYTES` / `MAX_EVENTS` constants. No
  single-row Node streaming in this change — only correct, consistent cap enforcement.

Deferred to later changes (not in scope): #25.1 Node `Readable`/`Writable`/`AsyncIterable`
bridge, #25.2 shared-formula member resolution.

## Capabilities

### New Capabilities
<!-- none — this change modifies existing streaming behavior, no new capability surface -->

### Modified Capabilities

- `streaming-xlsx`: requirement changes for (1) sheet file resolution by rels target
  path rather than digit-greedy filename parse (A1), (2) streaming size caps must bound
  *actual* decompressed bytes, not the declared size, to stay safe on untrusted input
  (A3 + #25.3), (3) empty cells must round-trip distinctly from empty-string cells
  (A5). A4's cell-boundary formula reset is already specified.

## Impact

- **Code**: `src/stream.rs` (`parse_workbook_sheet_targets`, return type and sheet-path
  construction; `parse_shared_strings` and `stream_read` to take `&mut ZipArchive`;
  size-guard sites to use `.take()`; `StreamValue::Empty` variant). `src/stream_handle.rs`
  (`from_js_value` / `to_js_value` to handle `Empty`).
- **API**: No change to the public napi surface signatures. `JsStreamValue` may gain an
  `empty: Option<bool>` (or document `{}`/`{value:null}` as empty) — additive, non-breaking.
- **Specs**: delta to `streaming-xlsx` for the three requirement changes above.
- **Tests**: new unit/integration tests for A1 (non-default filename), A3 (hostile
  declared-vs-real size), A4 (missing `</f>`), A5 (empty round-trip); A2 is a
  perf/refactor with no behavior change (guarded by existing round-trip tests).
