# Proposal

## Why

PR #73 review (`fix/small-parity-gaps`) surfaced two behavior defects, both reproduced and CONFIRMED by `@bug-hunter` on standalone verbatim-copy repros. A themed color and a plain-authored twin resolving to the same ARGB silently merge in the style table (first-wins), corrupting one of the two cells' rendering; and a crafted `<col min="0">` creates the reserved `col_num = 0` sentinel and round-trips as invalid OOXML (`min="0"`, schema requires `>= 1`). Both are silent data-corruption paths, not crashes, and both contradict the specs shipped by the change under review.

## What Changes

- Style-table dedup keys become theme-aware: two sub-table entries (font, fill, border) that differ only in theme linkage SHALL occupy separate slots, so a themed color and a plain ARGB twin never merge.
- `<col>` parsing clamps the lower bound: descriptors with `min < 1` are ignored (both the width/hidden parser and the outline-level parser, which share the pattern), so `col_num = 0` can never be created from file input.
- No public API change. No new file parts, no emission-policy change (theme-wins stands).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `theme-color-references`: dedup SHALL distinguish themed vs plain entries resolving to the same ARGB (font, fill foreground/background, border-side colors).
- `dimension-properties`: the `<cols>` reader SHALL NOT create column definitions with `col_num < 1`; descriptors with `min < 1` are clamped/ignored.

## Impact

- Affected code: `src/writer/styles.rs` (`canonical_key` / `build_style_table` dedup), `src/reader/xlsx.rs` (`parse_col_dims_from_xml`, `parse_col_outline_levels_from_xml`), tests in `src/writer/styles.rs`, `src/reader/xlsx.rs`, plus vitest coverage for the mixed twin case.
- Style-table indices shift for mixed themed/plain workbooks (dedup splits one slot into two); plain-only and themed-only files are unaffected.
- Crafted `min="0"` files now read without creating a sentinel column; previously-created column-0 definitions disappear on re-read.
