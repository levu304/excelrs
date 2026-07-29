## Context

PR #42 fixed merged-cell border rendering by filtering non-anchor cells from sheetData XML. The review confirmed a style-index desync bug (fixed in follow-up commit) and surfaced three structural issues:

1. `is_cell_merged_anchor()` on Worksheet is dead code — the writer reimplements the same parsing logic inline
2. Merge-range parsing (`split(':')` + `parse_address`) is duplicated across `is_merged()`, `is_cell_merged_anchor()`, and the inline filter in `write_cells_with_styles`
3. `parse_address` is called in the tight inner loop for every cell, re-parsing the same cell address string repeatedly

The existing `merge-range-writer` spec defines the requirement: non-anchor cells in merged ranges must be omitted from sheetData. The `is_cell_merged_anchor` helper was added to support this but is never called by the writer.

## Goals / Non-Goals

**Goals:**

- Extract a shared `parse_merge_range()` helper on `Worksheet` to eliminate parsing duplication across 3+ call sites
- Wire `write_cells_with_styles` to use `ws.is_cell_merged_anchor()` instead of the inline filter
- Fix the style-index desync bug (cell_si iterator must advance for every cell, even skipped ones)
- Add TDD regression test covering non-anchor merged cells WITH values and styles

**Non-Goals:**

- Changing the public API surface (no NAPI/TS changes)
- Modifying the reader path
- Changing how `written_cells()` works (still returns all cells including non-anchor merged cells)
- Performance optimization of `parse_address` caching (deferred to separate change if needed)

## Decisions

### D1: Shared helper `parse_merge_range()` on Worksheet

Extract `parse_merge_range(range: &str) -> Option<(u32, u32, u32, u32)>` as a private method on `Worksheet`. This replaces the duplicated `split(':')` + `parse_address` pattern in `is_merged()`, `is_cell_merged_anchor()`, and the writer inline filter.

**Rationale**: Single source of truth for merge range parsing. If `parse_address` behavior changes or merge range format evolves, only one place needs updating.

**Alternatives considered:**

- Free function in `types.rs` — rejected because it's only used in merge-related contexts and doesn't need to be public
- Inline the helper in each call site — rejected because it's the exact problem we're solving (duplication)

### D2: Writer calls `ws.is_cell_merged_anchor()` instead of inline filter

Replace the inline merge filter in `write_cells_with_styles` with a call to `ws.is_cell_merged_anchor(cell_row, cell_col)`. The writer already has access to `ws: &Worksheet`.

**Rationale**: Eliminates dead code (`is_cell_merged_anchor` was added but never used by the writer), consolidates logic, and ensures the writer uses the canonical implementation.

**Alternatives considered:**

- Keep inline filter but extract to a helper — rejected because `is_cell_merged_anchor` already exists and just needs to be wired in
- Remove `is_cell_merged_anchor` entirely — rejected because it's a useful public model method even if not currently used by the writer

### D3: Style iterator advances for every cell

Move `cell_si.next()` before the merge-range skip check so the iterator always advances exactly once per cell in `written_cells()`.

**Rationale**: `cell_style_indices` has one entry per cell in `written_cells()`. Skipping a cell without consuming its style index causes subsequent cells to receive wrong `s` attributes.

### D4: TDD regression test for non-anchor cells with values

Add a test that sets values AND styles on non-anchor merged cells (G3:K3), writes to XML, and asserts those cells are absent from sheetData. This is the exact regression scenario from the original bug report.

**Rationale**: The existing `test_merge_cells_border_only_xml` only tests non-anchor cells with NO values — those are already excluded from `written_cells()` by `is_effectively_empty()`. The real bug only manifests when non-anchor cells have content.

## Risks / Trade-offs

- **`is_cell_merged_anchor` now called from writer**: The writer holds a `&Worksheet` reference, so calling `ws.is_cell_merged_anchor()` requires no new dependencies. Risk is low.
- **`parse_merge_range` helper changes behavior of `is_merged` and `is_cell_merged_anchor`**: Refactoring the parsing logic into a shared helper should be behaviorally identical. The existing tests for both methods cover the parsing paths.
- **TDD test adds complexity**: The regression test requires setting up cells with both values and styles in a merged range, then inspecting raw XML. This is more complex than the existing border-only test but necessary to cover the actual bug scenario.

## Open Questions

- Should `parse_merge_range` be `pub(crate)` or private? Currently all merge-related methods are `pub` — keeping `parse_merge_range` private is sufficient since it's only used within `Worksheet`.
- Does the `parse_address` in the inner loop need caching? Currently called once per cell per write. For typical sheets (<1000 cells), the overhead is negligible. Defer optimization.
