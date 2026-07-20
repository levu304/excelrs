## Context

v2.1.0 delivered `stream_read` / `parse_sheet_rows` in `src/stream.rs` (per-entry
zip + quick-xml SAX, no `Workbook` model). It captures inline `<f>` text into
`StreamValue::Formula`, but does not read the `<f>` element's `t` / `si` / `ref`
attributes, so shared-formula *member* cells (no inline text) round-trip their
cached `<v>` value instead of a formula. The whole-workbook reader relies on
calamine 0.35.0, whose `next_formula` / `replace_cell_names` already expands
shared formulas; "same as the whole-workbook reader would" therefore has a precise,
existing definition we can port. The `streaming-xlsx` spec already mandates
"same fidelity as the whole-workbook reader on the read path," so this closes a
fidelity gap, deferred from #25.3 as #25.2 (GitHub #32).

Constraints discovered during exploration:

- calamine 0.35.0 source (`src/xlsx/cells_reader.rs`, `src/xlsx/mod.rs`) is the
  reference semantics. Master cells carry `ref` + inline text; members carry
  neither. calamine builds a per-`ref`-range `offset_map` keyed by every cell in
  the range — O(range) memory we do not need.
- The streaming reader already resets formula-capture state at the cell boundary
  (the #25.3 guard), so a malformed `</f>` cannot leak; the shared-formula table
  is independent of that flag.
- `si` is per-sheet scoped; `parse_sheet_rows` is called once per worksheet, so a
  local `HashMap` is correctly scoped with no cross-sheet leakage (unlike the
  `Arc<Vec<String>>` shared-strings table, which is global to the workbook).
- `StreamValue::Formula(String)` already exists and serializes through
  `index.d.ts`; no new public type is required.
- The `MAX_ENTRY_BYTES` / `MAX_EVENTS` resource contract must be preserved — the
  table adds only a tiny `HashMap` and zero extra SAX events.

## Goals / Non-Goals

**Goals:**

- Streaming read resolves shared-formula member cells to the same translated
  formula text the whole-workbook (calamine) reader produces.
- Master cells return their own formula; non-shared formulas are unchanged.
- Keep the streaming reader constant-memory: no whole-sheet materialization, no
  per-range `offset_map`; only a small per-sheet `si` table.

**Non-Goals:**

- Writing shared formulas (the writer already expands on write per v0.1; out of
  scope).
- Stricter-than-calamine translation of sheet-qualified references (`Sheet1!A1`)
  in v1 — accepted as a documented limitation.
- Cross-sheet shared-formula master lookup (members always live on the same
  sheet as their master; per-sheet scoping is correct).
- Any change to the public TS/JS API shape (reader-behavior fix only).

## Decisions

**D1 — Match calamine semantics exactly for v1 (over a stricter port).**
Port `replace_cell_names` faithfully, including its blind spot: a token that
fails to parse as a reference (e.g. `Sheet1!A1`) is copied verbatim and not
shifted. This guarantees "same as the whole-workbook reader" with low risk and
zero new heuristics. *Alternative considered:* parse and strip the sheet
qualifier, shift the bare reference, re-prefix — stricter but adds a tokenizer
branch and can still diverge from calamine on edge cases. Defer to a later change;
document the limitation.

**D2 — On-the-fly offset instead of calamine's per-range `offset_map`.**
calamine stores `HashMap<(row,col), (i64,i64)>` over the entire `ref` range. We
store only `si → (master_text, master_pos)` and compute
`offset = member_pos - master_pos` per member. Output is identical for every
valid file (a member is always within the master's `ref` range, so its position
minus the master position equals calamine's stored offset), but our memory is
bounded by the number of distinct shared formulas, not by range size. This also
removes calamine's "member outside `ref` range → no formula" quirk as a side
benefit (any member with a seen master resolves).

**D3 — State lives in `parse_sheet_rows` (per-sheet call).**
The `HashMap` is a local in `parse_sheet_rows`, torn down at the end of each
sheet. This is the natural scoping unit for `si` and keeps the function
self-contained (no new struct threaded through `stream_read`).

**D4 — Non-breaking: reuse `StreamValue::Formula`.**
No new enum variant, no new napi type, no `index.d.ts` change. The fix only makes
member cells populate the existing `Formula` variant. Existing streaming callers
see formulas where they previously saw cached numbers.

**D5 — Resolve at the `</c>` boundary via `formula_buf`.**
On a member `<f>`, when `si` resolves, write the translated text into
`formula_buf` (with `has_formula` already `true` from the `<f>` start). Then the
existing `build_cell_value` returns `StreamValue::Formula(translated)` and the
cached `<v>` value is replaced — one minimal wiring point, no new branch in the
value builder.

## Risks / Trade-offs

- **Master-after-member (malformed order).** If a member precedes its master in
  document order, the lookup misses and the member emits no formula. Valid files
  always write the master first (top-left of the `ref` range, row-major), so this
  only affects malformed input — and matches calamine, whose vector slot is still
  `None`. Acceptable; no special handling.
- **Sheet-qualified references not shifted (D1).** A shared formula whose relative
  refs are sheet-qualified (`Sheet1!A1`) will not be translated correctly in v1.
  This matches calamine and is documented as a known limitation; rare in practice.
- **Cached `<v>` replaced by formula on members.** This is the intended fix (the
  bug is that members currently return the cached value). Verified that the
  non-streaming reader likewise returns `Formula` for these cells, so fidelity
  improves, not diverges.
- **Resource contract.** The `HashMap` is bounded by distinct `si` per sheet
  (typically <10); no extra SAX events are emitted. `MAX_ENTRY_BYTES` /
  `MAX_EVENTS` are untouched.
- **TS surface already renders `Formula`.** `StreamValue::Formula` is already
  emitted for inline (non-shared) formulas and serialized to JS; this change only
  extends coverage. Confirm the JS round-trip test exercises a member cell.

## Migration Plan

Reader-behavior fix, additive. No API, type, or schema change. Existing streaming
readers start returning `Formula` for shared-formula members (previously numbers);
callers that relied on the cached value would need to handle `Formula` — but that
was a fidelity bug, and the spec already promises formula fidelity. Close GitHub
# 32 and #25.2; update ROADMAP's shared-formula row. No rollback beyond a normal
release.

## Open Questions

1. **Test oracle.** Assert streaming output equals the whole-workbook reader's
   output for the same file (true "same fidelity"), and/or call calamine directly
   in the Rust test? Recommend comparing against excelrs's own non-streaming read
   where feasible, plus a hand-checked fixture (`=A1+B1` at B2 shared to B10 → B5
   is `=A4+B4`).
2. **Strictness (D1).** Ship calamine-equivalent (recommended) now, or invest in
   sheet-qualifier-aware translation in this change?
3. **Negative-path test.** Should the malformed master-after-member case be an
   explicit test asserting "no formula, no panic" (matching calamine), or left to
   fuzzing?
