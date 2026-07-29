## Why

PR #42 fixed merged-cell border rendering by filtering non-anchor cells from sheetData XML. The review uncovered three confirmed issues: (1) style-index desync when `continue` skips `cell_si.next()`, (2) merge-range parsing duplicated across `is_merged()`, `is_cell_merged_anchor()`, and the inline writer filter, and (3) `is_cell_merged_anchor` is dead code since the writer reimplements its logic inline. This change consolidates the parsing, wires the writer to use the helper, and adds regression tests.

## Changes

- **Extract `parse_merge_range()` helper** on `Worksheet` — single source of truth for `split(':')` + `parse_address` pattern, used by `is_merged()`, `is_cell_merged_anchor()`, and the writer
- **Wire `write_cells_with_styles` to call `ws.is_cell_merged_anchor()`** — replace inline filter with helper call, eliminating dead code and duplication
- **Fix style-index desync** — ensure `cell_si.next()` advances for every cell in `written_cells()`, even skipped ones (regression fix from PR #42 review)
- **Add TDD regression test** — non-anchor merged cell WITH value and style is suppressed in XML output, verifying the exact bug scenario from the original report

## Capabilities

### New Capabilities

- `merge-range-parsing-consolidation`: Shared `parse_merge_range()` helper eliminates duplication across 3+ call sites and provides a single point of maintenance for merge range parsing logic

### Modified Capabilities

- `merge-range-writer`: Writer now uses consolidated `parse_merge_range()` helper and `is_cell_merged_anchor()` instead of inline duplicated logic; style-index desync bug fixed; regression test added for non-anchor cells with values
