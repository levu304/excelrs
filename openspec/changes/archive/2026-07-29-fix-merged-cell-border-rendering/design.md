## Context

The writer (`src/writer/xlsx.rs`) builds a per-sheet `cell_styles` vector by iterating `ws.rows()` → `row.written_cells()` and pushing each cell's effective style; `build_style_table` turns that into `cell_style_indices`. `write_cells_with_styles` then iterates the same `written_cells()` and emits each `<c>` with its index, advancing `cell_si` once per cell.

Commit `1b8b28b` added a filter in that loop: cells inside a merged range but not the top-left anchor are skipped (`continue`). The intent was to stop non-anchor cells from overriding the anchor's border. But `written_cells()` already drops empty cells (`is_effectively_empty()` in `src/model/row.rs:229`), so the empty non-anchors in a typical merge (e.g. `G3:K3` when only `F3` is styled) were **never emitted in the first place** — the skip is a no-op for the common case and the bug persists.

The real fix (matching ExcelJS): emit the **entire merge bounding box**, not just the anchor. Excel needs the non-anchor cells present in `<sheetData>` to draw the anchor's border/formatting across the whole range.

Current relevant surfaces:

- `write_cells_with_styles` (~`src/writer/xlsx.rs:1703`): iterates `written_cells()`, advances `cell_si`, writes `<c>`. Currently skips non-anchor merged cells.
- Style-index build loop (top-level `workbook_to_bytes`, ~`src/writer/xlsx.rs:160`): iterates `written_cells()` only.
- `ws.get_merged_ranges()` → `Vec<String>` (e.g. `["F3:K3"]`); `parse_merge_range` → `(c1,r1,c2,r2)`; `is_cell_merged_anchor(row,col)` exists.

## Goals / Non-Goals

**Goals:**

- Emit every cell in every merged range's bounding box (anchor + non-anchors) into `<sheetData>`, each with its effective style.
- Style-index iterator stays aligned (no desync for cells after a merge range).
- Excel renders the anchor's borders/formatting across the full merged range (verified by equivalence to ExcelJS output).
- No JS/TS API changes.

**Non-Goals:**

- No reader changes — reader already round-trips merges correctly.
- No change to `Row::written_cells()` contract (shared-string builder, hyperlink collector, dimension calculator still use it).
- No support for overlapping/invalid merges (OOXML-invalid; leave undefined).

## Decisions

### Decision 1: Inject empty non-anchor merged cells into the emitted cell set, not into the model

**Chosen**: Compute, per worksheet, the set of merge-bounding-box addresses that are *not* already in `written_cells()` (the empty non-anchors). Add them to both the style-index build and the emission loop, each with its effective style.

**Alternatives considered:**

1. **Filter in `Row::written_cells()`** — rejected: changes the model contract; other callers (shared strings, hyperlinks, dimension) need merge-shadow cells. The merge expansion is purely an XML-emission concern.
2. **Prune cells from the model at merge time** — rejected: destructive, breaks reader round-trip where non-anchor cells should reappear.

### Decision 2: Build style indices from the *expanded* cell set

**Chosen**: The top-level style build loop collects `written_cells()` **plus** the injected empty non-anchor addresses; their effective style is `None` (Normal) unless a column style applies at that column, in which case the column style is used. This keeps `cell_style_indices` and the emission loop 1:1, so the existing `cell_si` iterator stays aligned with no skip needed.

**Alternatives considered:**

1. **Hardcode `s="0"` for injected cells during emission, bypassing the index vector** — simpler, but wrong when a styled column underlies a merged non-anchor cell (ExcelJS would emit the column style there). Expanding the index set is the faithful fix.

### Decision 3: Emit injected cells as empty `<c r="ADDR" s="N"/>` (no `<v>`)

**Chosen**: Injected empty non-anchors carry only an address + style index, matching ExcelJS (`<c r="G3"/>` / `<c r="G3" s="0"/>`). The emission loop merges injected addresses into the per-row cell sequence, sorted by column, and emits them with no value.

**Alternatives considered:**

1. **Materialize full `Cell` objects for empty non-anchors** — unnecessary allocation; an address + style index is enough for emission.

### Decision 4: Remove the non-anchor skip filter

**Chosen**: Drop the `is_merged && !is_cell_merged_anchor` `continue` added by `1b8b28b`. All cells in the expanded set are now emitted (anchors keep their real style; injected empties get Normal/column style).

**Alternatives considered:**

1. **Keep the filter and also inject** — contradictory; removing is cleaner and aligns the writer with ExcelJS.

## Risks / Trade-offs

- **[Behavior] Existing tests assert non-anchors ABSENT** (`test_merged_range_with_values_omitted`, `test_style_index_sync_after_merged_cells`) → must be rewritten to assert PRESENCE; add a regression test for the merged-border XML.
- **[Edge] Column style underlies a merged non-anchor** → covered by effective-style computation (Decision 2); emits the column style index, matching ExcelJS.
- **[Edge] Non-anchor cell has its own value/style** → already in `written_cells()`, emitted with its real style; only genuinely empty non-anchors are injected (no double-emit).
- **[Perf] Large merge bounding box** (e.g. `A1:Z100` = 2600 cells) adds that many `<c>` elements. Matches ExcelJS behavior; real sheets rarely merge that large. Acceptable; note as known cost.
- **[Risk] Overlapping merges** → OOXML-invalid; behavior undefined. Skip.

## Migration Plan

- The writer change is internal; output becomes ExcelJS-equivalent. No consumer-visible API change.
- Rollback: revert the writer change + restore the two tests; the `merge-range-writer` spec delta is archival-only.

## Open Questions

- None outstanding — the ExcelJS output is the confirmed ground truth for correct rendering.
