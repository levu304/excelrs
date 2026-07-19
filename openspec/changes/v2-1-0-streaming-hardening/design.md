## Context

v2.0.0 (PR #24) shipped the streaming XLSX reader/writer in `src/stream.rs` (Rust
core) + `src/stream_handle.rs` (napi FFI). A post-merge qa-expert audit (issue #26)
verified five residual risks A1–A5; issue #25 tracked deferred limitations. This
change is the **hardening** subset: correctness, security, and fidelity fixes that
are low-risk and add no new public surface, so they belong in a point release
(v2.1.0) before the larger #25.1 (Node stream bridge) and #25.2 (shared-formula
resolution) features.

Current state relevant to each item:

- **A1**: `parse_workbook_sheet_targets` returns `(name, num)` where `num` comes from
  `sheet_number_from_target` — a digit-greedy collect of all digits in the filename.
  Used to reconstruct `xl/worksheets/sheet{num}.xml`. Breaks on non-default filenames
  (`sheet_v2.xml` → `2`) and when a filename number disagrees with document order.
- **A2**: `stream_read` builds a `ZipArchive` (L106), then `parse_workbook_sheet_targets`
  (L142) and `parse_shared_strings` (L239) each build their own from the same bytes;
  style parsing builds another. 3–4 opens per call, all re-parsing the central directory.
- **A3**: every streamed entry is guarded by `if entry.size() > MAX_ENTRY_BYTES` then
  `read_to_string`. `entry.size()` is the *declared* uncompressed size — a hostile zip
  can lie, decompress far more, and OOM. (`reader/styles.rs` L203/L759 already use the
  correct `.take(MAX)` pattern.)
- **A4**: already fixed in v2.0.0 — `in_f` resets at the `</c>` cell boundary
  (`stream.rs` L422, `ponytail:` marker) and the spec already requires it. No code change.
- **A5**: `from_js_value` maps an empty JS cell to `StreamValue::Text("")`, collapsing
  empty vs `""`. `StreamValue` has no empty representation.
- **#25.3**: caps are `MAX_ENTRY_BYTES = 16 MiB`, `MAX_EVENTS = 5M`; enforcement is
  inconsistent (declared-size only) and not documented as the streaming contract.

## Goals / Non-Goals

**Goals:**

- Correctly resolve sheet files regardless of filename (A1).
- Remove redundant zip I/O in the streaming read path (A2, scoped — see below).
- Bound *actual* decompressed bytes on untrusted input (A3), applied consistently (#25.3).
- Preserve empty-cell distinction across the FFI round-trip (A5).
- Verify A4 with a regression test (no code change).
- No public API break; only an additive FFI field.

**Non-Goals:**

- Node `Readable`/`Writable`/`AsyncIterable` bridging (#25.1) — separate change.
- Shared-formula member resolution (#25.2) — separate change.
- Single-row streaming of arbitrarily huge sheets / changing cap values (#25.3's
  "lower the caps" option) — out of scope; we fix *correctness* of caps, not their magnitude.
- Reusing the style-parse `ZipArchive` (see A2 decision) — deferred; keeps blast radius in `stream.rs`.

## Decisions

### A1 — Resolve sheet file by rels target path

Change `parse_workbook_sheet_targets` to return `Vec<(String, String)>` where the
second element is the resolved zip path (`format!("xl/{}", rels_target)`, since rels
targets are relative to `xl/`). `stream_read` opens `archive.by_name(&path)` directly
and removes `sheet_number_from_target` entirely.

- **Why**: the rels file already maps `r:id → worksheets/sheetN.xml` (the real file).
  Re-deriving `sheetN.xml` from filename digits is redundant and fragile; using the
  target directly is correct even when filenames don't match document order.
- **Alternatives**: (a) tighten the regex to `sheet(\d+)\.xml$` — still returns `None`
  for `sheet_v2.xml`, dropping the sheet; (b) keep digit parse but re-order by `r:id` —
  more code, same fragility. Direct target use is shortest and most correct.

### A2 — Reuse one `ZipArchive` across stream.rs helpers

Open the archive once in `stream_read`; pass `&mut ZipArchive<Cursor<&[u8]>>` to
`parse_workbook_sheet_targets` and `parse_shared_strings`. The per-sheet loop already
shares the `stream_read` archive.

- **Why**: each `ZipArchive::new` re-parses the central directory — pure redundant I/O.
  This removes 2 of the 3 stream.rs-local opens per call (down to 2 total: the
  `stream_read` archive + the style-parse archive).
- **Scope cut (surgical)**: the style-parse open inside
  `reader_styles::parse_styles_and_sheet_maps` is left as-is. Threading `&mut ZipArchive`
  through it would change a module shared with the whole-workbook reader
  (`reader/xlsx.rs`), widening blast radius for marginal gain. Deferred.
- **Alternatives**: leave all separate (no win); thread through styles too (bigger diff,
  shared-module risk).

### A3 — Bound actual bytes via `.take()`, keep friendly declared-size error

At each streamed entry, keep the `if entry.size() > MAX_ENTRY_BYTES` check to emit the
clear "exceeds streaming size limit" error for *legitimately* oversized entries, then
read with `entry.take(MAX_ENTRY_BYTES).read_to_string(&mut s)?` so a *hostile* declared
size cannot decompress past the cap.

- **Why**: `entry.size()` is attacker-controlled; `.take()` bounds the real stream
  (matches the already-correct `reader/styles.rs` pattern). Keeping the declared-size
  check preserves a good error message for the common benign case.
- **Alternatives**: drop the declared check (lose friendly message); trust `entry.size()`
  (vulnerable). Both rejected.

### A4 — No code change; add regression test

- **Why**: already shipped in v2.0.0 (`in_f` reset at `</c>`, spec requirement merged in
  `09c6164`). Confirmed by reading current `stream.rs`.

### A5 — Add `StreamValue::Empty` variant

Add `Empty` to `StreamValue`. `from_js_value` returns `Empty` when no field is set;
`to_js_value` emits `JsStreamValue { empty: Some(true), .. }` (additive
`empty: Option<bool>` field). The writer emits a cell with no `<v>` element.

- **Why**: Excel/ExcelJS distinguish an empty cell from a cell holding `""`; the current
  `Text("")` mapping loses that on round-trip.
- **Alternatives**: wrap values in `Option<StreamValue>` (churns every call site) —
  rejected as too broad for one distinction.

### #25.3 — Consistent cap enforcement + documentation

Apply the A3 declared-check + `.take()` pattern to every streamed entry (workbook.xml,
its rels, each sheet, sharedStrings.xml). Document `MAX_ENTRY_BYTES` / `MAX_EVENTS` as
the streaming contract in `design.md` / code comments. Values unchanged.

## Risks / Trade-offs

- **[A2 partial]** → Style-parse still opens its own archive; a 3rd open remains.
  Mitigation: acceptable; documented as deferred, tracked if profiling shows it matters.
- **[A3 `.take()` truncation]** → a hostile-but-truly-huge legit file is silently
  truncated, then fails to parse. Mitigation: the declared-size check fires first for
  benign oversize with a clear message; truncation only triggers on a lying declared size.
- **[A5 additive field]** → `JsStreamValue.empty` is new; old callers simply omit it.
  Mitigation: `Option<bool>`, non-breaking.
- **[A1 path construction]** → rels targets are normally relative (`worksheets/sheet3.xml`);
  an absolute or `../` target is unusual. Mitigation: prefix only when the target is not
  already `xl/`-prefixed; existing fallback (`Sheet1` → `xl/worksheets/sheet1.xml`) preserved.

## Migration Plan

- No public API break. `JsStreamValue.empty` is additive.
- Ship in v2.1.0. Rollback = revert the commit; no data/schema migration.
- Existing streaming round-trip tests guard A2 (no behavior change) and A5 (empty
  round-trip) automatically.

## Open Questions

- None blocking. (Style-parse archive reuse and cap *value* changes are explicitly
  deferred, not questions.)
