## Purpose

Consolidate merge-range parsing into a single shared helper to eliminate duplication across `is_merged()`, `is_cell_merged_anchor()`, and the inline filter in `write_cells_with_styles`.

## Requirements

### Requirement: `parse_merge_range()` helper parses merge range strings

The `Worksheet` model SHALL provide a private helper `parse_merge_range(range: &str) -> Option<(u32, u32, u32, u32)>` that splits a merge range string on `:`, parses both endpoints via `parse_address`, and returns `(anchor_col, anchor_row, end_col, end_row)`.

#### Scenario: Valid merge range parses correctly

- **WHEN** calling `parse_merge_range("F3:K3")`
- **THEN** returns `Some((6, 3, 11, 3))`

#### Scenario: Invalid merge range returns None

- **WHEN** calling `parse_merge_range("invalid")` (no colon separator)
- **THEN** returns `None`

#### Scenario: Malformed address in range returns None

- **WHEN** calling `parse_merge_range("F3:INVALID")`
- **THEN** returns `None`

### Requirement: `is_merged()` uses `parse_merge_range()`

The `is_merged(row, col)` method SHALL use `parse_merge_range()` internally instead of duplicating the split+parse logic.

#### Scenario: `is_merged` delegates to helper

- **WHEN** `is_merged(3, 7)` is called on a worksheet with merge range `F3:K3`
- **THEN** it returns `Some("F3:K3")` using the shared `parse_merge_range()` helper

### Requirement: `is_cell_merged_anchor()` uses `parse_merge_range()`

The `is_cell_merged_anchor(row, col)` method SHALL use `parse_merge_range()` internally instead of duplicating the split+parse logic.

#### Scenario: Anchor detection uses helper

- **WHEN** `is_cell_merged_anchor(3, 6)` is called on a worksheet with merge range `F3:K3`
- **THEN** it returns `true` using the shared `parse_merge_range()` helper

#### Scenario: Non-anchor detection uses helper

- **WHEN** `is_cell_merged_anchor(3, 7)` is called on a worksheet with merge range `F3:K3`
- **THEN** it returns `false` using the shared `parse_merge_range()` helper
