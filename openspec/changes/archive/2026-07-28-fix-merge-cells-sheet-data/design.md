## Context

Writer (`src/writer/xlsx.rs`) emits every cell from every row into `<sheetData>` via `write_cells_with_styles`. The `<mergeCells>` declaration is emitted correctly, but non-anchor cells within merged ranges still appear with `s="0"` (Normal style, no border). Excel resolves the conflicting signals by using the per-cell style over the merge-range inheritance, so only the anchor cell's border is rendered.

Current relevant code surfaces:

- `write_cells_with_styles` (line ~1700): iterates `ws.rows()` → `row.written_cells()` → `write_cell_xml`. No merge-awareness.
- `ws.get_merged_ranges()` returns `Vec<String>` e.g. `["F3:K3"]`.
- `write_sheet_xml` calls `ws.get_merged_ranges()` to emit `<mergeCells>` — same data available, but not used during cell emission.

## Goals / Non-Goals

**Goals:**

- Non-anchor cells within any merged range are **omitted** from `<sheetData>` XML.
- Anchor cell (top-left of each merge range) IS emitted with its full style.
- All existing tests pass unchanged.
- No API surface changes (JS/TS side untouched).

**Non-Goals:**

- No changes to the reader path — reader already handles merged cells correctly.
- No changes to the in-memory model — `Row.written_cells()` continues returning all cells.

## Decisions

### Decision 1: Filter in `write_cells_with_styles`, not in `written_cells()`

**Chosen**: Add a filter lambda inside `write_cells_with_styles` that checks each cell against the worksheet's merge ranges before writing.

**Alternatives considered:**

1. **Filter in `Row::written_cells()`** — would change the model's contract; other callers (shared string builder, hyperlink collector, dimension calculator) also iterate `written_cells()` and should still see merge-shadow cells for those purposes. The merge filter is purely an XML-emission concern.
2. **Prune cells from model at merge time** — destructive; breaks reader round-trip where we want non-anchor cells to reappear.

### Decision 2: Parse merge ranges into (col1, row1, col2, row2) once per sheet

**Chosen**: Before the `write_cells_with_styles` cell loop, parse all merge ranges into a `Vec<(u32, u32, u32, u32)>`. The cell-check loop then does O(ranges) integer comparisons per cell — negligible for typical <100 ranges.

**Alternatives considered:**

1. **Parse on every cell check** — wasteful string splitting per cell.
2. **BTree interval tree** — overengineered; actual merge ranges per sheet rarely exceed 50.

### Decision 3: `is_merged_anchor` helper on `Worksheet`

**Chosen**: Add a method `fn is_cell_merged_anchor(&self, row: u32, col: u32) -> bool` that returns true only for the anchor (top-left) cell. The writer uses this to skip non-anchor cells.

**Alternatives considered:**

1. **`is_merged` already exists** but returns `Option<String>` for any cell in a merged range. Not directly usable — need the anchor check.
2. **Filter set of skip addresses** — precompute `HashSet<String>` of non-anchor addresses. Works but requires address string allocation.

### Decision 4: Preserve `s="0"` cells that are NOT in any merge range

Normal cells outside merge ranges continue emitting with `s="0"`. No change to existing behavior.

## Risks / Trade-offs

- **[Edge Case] Nested/overlapping merge ranges**: Not supported by OOXML — skip. The `is_merged_anchor` check naturally handles this since overlapping ranges are invalid anyway.
- **[Performance] String parsing per merge range**: Minimal — merge range count is typically <10 per sheet. Parsed once per sheet write.
- **[Behavior Change] Test `test_normal_cell_has_s_attr`**: Uses row data that may not overlap with merges in that test case, but must be re-verified after change.
- **[Behavior Change] `test_round_trip_merge_cells`**: Currently writes anchor value + merge range. After change, non-anchor cells still write correctly because they had no value. No change expected.
