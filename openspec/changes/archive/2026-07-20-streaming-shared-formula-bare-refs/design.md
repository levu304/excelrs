## Context

The streaming shared-formula resolution (PR #35, change
`streaming-shared-formula-resolution`) ports calamine 0.35.0's
`replace_cell_names` into `src/stream.rs` as a free function plus the `Ref`
enum (`Cell` / `Row` / `Column`) with `parse` / `offset` / `format`. Reference
translation is driven by `offset_ref_token`, which splits a token on `:` (so
`A1:A3` ranges both shift) and, for a single token, requires
`matches!(r, Ref::Cell { .. })` before shifting. That guard is the bug: a bare
`A` parses to `Ref::Column` and a bare `5` to `Ref::Row`, both hit the guard and
are emitted verbatim, whereas calamine shifts them. The whole-workbook reader
therefore differs from the streaming reader for shared formulas that reference a
whole column or row without a range (e.g. `=SUM(A)`, `=A+B`, `=A1*5`).

Constraints:

- The streaming reader's acceptance bar for shared formulas is "same as the
  whole-workbook (calamine) reader" (design **D1** of the parent change). calamine
  shifts bare columns and rows, so the streaming reader must too.
- `Ref::offset` ends in `validate()`, which rejects `col >= SF_MAX_COLUMNS ||
  row >= SF_MAX_ROWS`. Any token whose column number overflows 16,384 (function
  names like `COLUMN`, `SUM`, `ROWS` — all-letter strings) fails validation and is
  copied verbatim. This is the *same* mechanism calamine uses to avoid shifting
  function names, so removing the `Ref::Cell` guard cannot turn a function name
  into a shifted reference.
- The fix is a one-line deletion in one function; no new types, no new SAX events,
  no public API change. The `MAX_ENTRY_BYTES` / `MAX_EVENTS` contract is
  untouched.

## Goals / Non-Goals

**Goals:**

- Bare column and row references inside shared-formula master text shift by the
  member offset, matching calamine / the whole-workbook reader.
- Function names and quoted strings continue to be left verbatim (no regression).
- Keep the change to the single existing `offset_ref_token` path; no new
  translation logic.

**Non-Goals:**

- Revisiting the `Sheet1!A1` sheet-qualified blind spot (still deferred, D1 of the
  parent change).
- Any change to the shared-formula master/member table, `StreamValue::Formula`
  wiring, or the per-sheet `HashMap`.
- Any public API shape change.

## Decisions

**D1 — Remove the `Ref::Cell` guard; rely on `validate()` for safety.**
The single-token branch becomes `let r = Ref::parse(token)?; r.offset(offset)?.format(buf);
Some(())`. Column/Row tokens shift through the same path as Cell. The column
bound in `validate()` is the only gate needed to keep function names verbatim —
and it is identical to calamine's behavior, so we preserve fidelity rather than
introduce a divergent heuristic.

**D2 — Unit-test at the `replace_cell_names` boundary.**
The parent change already compares streaming output against the whole-workbook
reader on a `Cell`-ref fixture; that fixture does not exercise bare column/row
refs, so the gap was invisible. Adding direct `replace_cell_names` assertions for
`=A+B` → `=B+C` and `=A1*5` → `=A2*6` pins the calamine behavior at the unit level
without a new binary fixture (YAGNI for a one-line shift).

## Risks / Trade-offs

- **Caliber of parity.** After this fix the streaming reader matches calamine for
  the full reference grammar *except* the still-deferred `Sheet1!A1` sheet-qualified
  case. That remaining divergence is documented and accepted; this change does not
  expand scope to it.
- **No new fixture file.** Tests assert on `replace_cell_names` directly rather
  than a `.xlsx` round-trip. This is sufficient: the function is the single source
  of reference rewriting, and an end-to-end bare-ref fixture would duplicate the
  parent change's fidelity test for a different token shape. If a reviewer wants a
  streaming-vs-whole-workbook fixture, it can be added later.
- **Stacking.** The edit lands in `offset_ref_token`, introduced by PR #35.
  Implement on top of (or immediately after) PR #35 so the two diffs compose.

## Migration Plan

Reader-behavior fix, additive, non-breaking. No API, type, or schema change. The
streaming reader now returns the *correct* (calamine-matching) formula text for
bare column/row member cells where it previously returned an unshifted string.
Update the PR #35 review comment (discussion_r3613652786) to note the fix, and
leave the parent change's ROADMAP row as-is (the feature was already marked
shipped). No rollback beyond a normal release.

## Open Questions

1. **Branch target.** Implement on `feat/streaming-shared-formula-resolution`
   (stacking on PR #35) or on `main` after it merges? Recommend stacking so the
   `offset_ref_token` history reads as one continuous fix.
2. **Fixture vs unit.** Is the `replace_cell_names` unit test enough, or does the
   merge gate require an end-to-end streaming-vs-whole-workbook bare-ref fixture?
