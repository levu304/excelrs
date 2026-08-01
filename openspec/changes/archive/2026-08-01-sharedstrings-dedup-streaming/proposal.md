# sharedstrings-dedup-streaming — Proposal

## Context

`src/stream.rs` `StreamSession` implements an **inline shared-string interner**:
`string_indices: HashMap<String, u32>` (session-level, not reset per sheet),
accessed via `entry(s).or_insert_with(...)` in `write_sheet_xml`. The same
`StreamSession` is reused across all sheets in `stream_write` and
`stream_write_to_file` (one session → loop `write_sheet_xml` → `finalize`),
so the dedup is **cross-sheet** today.

The deferred cap-`streaming-write-incremental` spec (Scenario: "same string in
sheet 1 and sheet 2 → `sharedStrings.xml` contains that string exactly once and
both sheet XML files reference it via the same index") is therefore **already
satisfied by code**, but it is **not asserted by any test**. The only dedup
test, `test_write_shared_string_dedup`, targets the whole-workbook writer
`src/writer/xlsx.rs` (confirmed via GitNexus). A regression to the streaming
interner would pass silently.

## Problem

> "per-sheet sharedStrings dedup during streaming"

Interpretation note (kept explicit → Rule 1): "per-sheet" here means **as each
sheet streams**, reusing the session-wide interner — i.e. a guarantee that
duplicate string *values* across sheets (and within a sheet) collapse to a
single `sharedStrings.xml` entry. It does **NOT** mean resetting the interner
per sheet (that would regress the existing cross-sheet behavior and is not valid
XLSX — the shared-strings table is workbook-global).

Because the behavior already exists, the real gap is a **missing guarantee +
test**, not missing code.

## Goals

- Lock in the existing inline cross-sheet dedup as a tested, documented
  guarantee of the streaming write path.
- Align the `streaming-write-incremental` spec with what the code already does
  (close the "Deferred — not satisfied" gap for the interning scenario).

## Non-Goals

- **Not** re-architecting the JS-bridge buffer in `src/stream_handle.rs`
  (`StreamWriter.sheets: Vec<StreamSheet>`). PR #49 already added cap-16
  backpressure and fixed the `spawn_blocking` panic; the residual per-session
  buffer is tracked by ADR-005 / issue #25.1 and is the reverted-#34 (`c19a4fc`)
  Path-A spike. Out of scope.
- No change to `sharedStrings.xml`/`styles.xml` emission ordering or byte output.

## Approach

- Add an integration test in `src/stream.rs` (near existing stream tests):
  write two sheets sharing a string value, then assert
  `sharedStrings.xml` `uniqueCount`/`count` equal the number of distinct
  strings and that both sheets reference the **same** `<c t="s"><v>idx</v>`.
  Reuse `parse_shared_strings` to read the produced table.
- Add a delta requirement to `specs/streaming-write-incremental/spec.md`
  (Modified capability) pinning the dedup guarantee; leave Status "Deferred"
  for the bridge buffer (unchanged — only the interning scenario is satisfied).

## Risks / Trade-offs

- Memory: `string_indices` HashMap is `O(distinct strings)` — constant-memory
  friendly and consistent with ADR-005 (no second sheet buffer introduced).
- Test parsing the produced zip: reuse `zip::ZipArchive` over the `Vec<u8>`
  returned by `stream_write` — same pattern the whole-workbook dedup test uses.

## Open Questions

- Should this test also assert the `count` attribute counts total occurrences?
  (Keeps it loose.) Default: assert `uniqueCount == distinct` only.
