## Why

The streaming shared-formula resolution (#25.2, PR #35) shifts `Cell` references
(`A1`) and `Cell` ranges (`A1:A3`) in shared-formula *member* cells, but leaves
**bare column / row** references (`A`, `5`) verbatim. The cause is the
single-token branch of `offset_ref_token` in `src/stream.rs`, which requires
`matches!(r, Ref::Cell { .. })` and returns `None` (verbatim) for any
`Ref::Column` / `Ref::Row` token. calamine 0.35.0's `replace_cell_names` — the
oracle the streaming reader is meant to match — shifts bare columns and rows too,
so a shared formula like `=SUM(A)` or `=A+B` resolves to *different* text between
`read()` (whole-workbook) and `stream()` (streaming). That breaks the
`streaming-xlsx` spec's "same fidelity as the whole-workbook reader on the read
path" mandate, and the change's own design decision **D1** ("match calamine
exactly"). It is a fidelity gap, not a new feature.

Surfaced by code review on PR #35 (discussion_r3613652786). The fix is a single
removal of the `Ref::Cell` guard; it is safe because `Ref::offset` already calls
`validate()`, which rejects any reference whose column exceeds `SF_MAX_COLUMNS`
(16,384). calamine's own function-name handling relies on the same fact: a token
like `COLUMN` parses to a column far above 16,384, fails validation, and is copied
verbatim — so dropping the guard cannot corrupt function names, it only starts
shifting the genuine bare `A` / `5` references calamine already shifts.

## Changes

- **Drop the `Ref::Cell` guard in `offset_ref_token`.** The single-token branch
  routes `Ref::Column` / `Ref::Row` through the same `offset` → `format` path as
  `Cell`, so bare column/row refs shift by the member offset like everything else.
  The existing `validate()` column bound keeps function names (`COLUMN`, `SUM`,
  `ROWS`) verbatim, preserving the prior change's function-name behavior.
- **Add unit tests for bare reference shifting.** Assert
  `replace_cell_names("=A+B", (1, 0)) == "=B+C"` (bare column) and
  `replace_cell_names("=A1*5", (1, 0)) == "=A2*6"` (bare row), pinning the calamine
  behavior at the unit level.
- **Closes the gap flagged on PR #35** and completes #25.2's "same as
  whole-workbook reader" promise for the full reference grammar, not just the
  `Cell` subset.

## Relationship to PR #35

This is a follow-up to the streaming-shared-formula-resolution change (PR #35),
which introduced the guarded `offset_ref_token`. Because it edits the same
`src/stream.rs` code, implementation should stack on top of PR #35 (branch
`feat/streaming-shared-formula-resolution`) — or land immediately after it merges —
so the two diffs compose into one coherent `offset_ref_token`.
