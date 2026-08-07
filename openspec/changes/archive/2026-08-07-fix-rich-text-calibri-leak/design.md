## Context

`src/reader/xlsx.rs` parses rich-text runs in two functions:
`parse_inline_str_rich_text_with` (inline `<is><r>` runs) and
`parse_shared_string_rich_text` (`<si><r>` runs from
`xl/sharedStrings.xml`). Both seed each run's font with `Font::default()`
(which sets `name = "Calibri"`, `size = 11`) and rely on `apply_rpr_child` to
overwrite fields from the run's `<rPr>` children. `apply_rpr_child` only sets
`font.name` when an `<rFont>` element is present. See proposal.md — Why.

## Goals / Non-Goals

- Goal: a rich-text run with no `<rFont>` reads back with `font.name === null`,
  not `"Calibri"`.
- Non-Goal: do not change writer behavior; do not change the font-color/theme
  resolution added in 1f55235.

## Decisions

1. **Seed an empty font, not `Font::default()`.** At each run start
   (`src/reader/xlsx.rs:2447` and `:2554`) set `current_font` to a font with all
   fields `None` (clear `name` and `size` after `Font::default()`, since
   `Font::default()` populates both).
   - Alternative considered: keep `Font::default()` but only clear `name` —
     rejected because `Font::default()` also sets `size = 11`, which would
     likewise leak `<sz val="11"/>` for runs that only set bold/italic.
     Clearing both matches OOXML "inherit cell default" semantics.
2. **No writer change.** Writer already emits `<rFont>`/`<sz>` only when the
   field is `Some`, so a `null`-name run is written correctly without an
   explicit font.

## Risks / Trade-offs

- [Run with no `<rPr>` at all] → `has_rpr` stays false → `font` already `null`.
  Unaffected by this change.
- [Files round-tripped before this fix] → a workbook saved by an earlier build
  may have stored a spurious `Calibri` where none existed. After the fix,
  re-reading such a file yields `null` (correct). Acceptable: the previous
  `Calibri` was spurious.

## Migration Plan

N/A — behavior fix on read; no stored state, no API change. Add a regression
test asserting the no-`<rFont>` run reads back `font.name === null`.

## Open Questions

- None.
