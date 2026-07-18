## Context

The writer already emits both constructs the reader currently ignores:

- `Worksheet::merge_cells(range)` exists (model) and the writer serializes `<mergeCells>` (since v0.5.0). Reading a merged workbook back drops every merge — there is no read-side parsing, no non-master-cell handling, and no `Merge` value representation.
- `Row.style: Arc<Mutex<Option<Style>>>` exists with a `Row::set_style` / `Row::style` pair, and the writer emits `<row s="N">` from that field. The reader never restores it, so row-level styling is lost on read.

The two release decisions made while triaging issue #3 — link partial fixes with `Refs` (not `Closes`); the real `Fill` field is `fill.foreground` (no `fill.color`) — were never written down, so they can regress.

Cell styles are already resolved on read via `style_table.resolve_style(xf_idx)` → `ws.insert_cell_style(row, col, style)` (reader/xlsx.rs:1780). Row styles reuse the same `style_table`; we only need to wire the `<row s="…">` attribute through it.

## Goals / Non-Goals

**Goals:**

- Reader parses `<mergeCells>` and restores merged ranges (worksheet-level collection) plus a minimal anchor representation.
- Reader parses `<row s="N">` and sets `Row.style` (mirrors the writer).
- Add `CONTRIBUTING.md` documenting the two release decisions.
- Extend the v1.2.1 release smoke test to round-trip a merge + a row style through the read path.

**Non-Goals:**

- Not adding new fill/border/theme features (those shipped in v0.6.0–v0.13.0).
- Not changing the writer behavior for merges or row styles (already correct).
- Not re-architecting the style pipeline.

## Decisions

- **Merges: worksheet-level collection is the source of truth.** The writer already serializes merges from a worksheet-level collection. The reader will populate that same collection (confirmed/fixed during implementation — field name TBD, e.g. `merges`/`merged_ranges`) and, for ExcelJS parity, set the top-left anchor cell's value to a minimal `Merge(range)` representation. **Why:** keeps a single serialization source; avoids a divergent per-cell merge model. The `<mergeCells>` element alone fully drives the round-trip, so the per-cell variant is parity sugar, not required for correctness.
- **Row style mirrors cell style.** Add an `insert_row_style(ws, row, style)` that calls `style_table.resolve_style(row_xf_idx)` and sets it via `Row::set_style` — the same pattern as `insert_cell_style`. **Why:** reuses the proven style-resolution path instead of inventing a second one.
- **Policy doc = `CONTRIBUTING.md` at repo root.** **Why:** standard location for contributor/PR/release conventions; no existing `CONTRIBUTING.md` or `RELEASE` doc to conflict with.
- **Smoke-test extension is additive.** The v1.2.1 test already writes a styled workbook and reads it back asserting `font.bold` + `fill.foreground`; we add a merge range + a row style and assert both survive. **Why:** extends the regression catcher to the two new read paths without changing the existing assertions.

## Risks / Trade-offs

- **[Risk]** Non-master cells inside a merge range may be absent from `<sheetData>` or carry an empty value. → *Mitigation:* only the anchor carries the value; for other cells in the range, create no phantom value (matches writer output). Verify with a round-trip test on a multi-cell merge.
- **[Risk]** Row `s` index may reference a style not yet resolved when the row element is parsed. → *Mitigation:* resolve through the already-built `style_table` (same map used for cell styles); row elements are parsed after styles in the current reader order.
- **[Risk]** `CellValue::Merge` variant shape is unconfirmed (enum location/name pending). → *Mitigation:* worksheet-level merge collection alone guarantees round-trip; the variant is added only for parity and can be scoped to a single minimal variant if the enum is awkward.

## Migration Plan

Additive read-side + docs. No schema/API break. Rollback = revert the commit; published `1.2.1` artifact is unaffected. Bump `package.json` to `1.2.2` and tag `v1.2.2` after merge (same tag-driven flow as v1.2.1).

## Open Questions

- Exact worksheet field that stores merges (grep didn't surface `merges` on `Worksheet` — confirm where `merge_cells` persists and the writer reads it from).
- Whether a `CellValue::Merge` variant is required for the round-trip or worksheet-level merges suffice (lean: worksheet-level suffices; variant is parity-only).
