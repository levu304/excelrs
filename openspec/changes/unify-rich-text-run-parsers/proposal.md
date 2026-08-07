## Why

Two near-identical rich-text run parsers live in `src/reader/xlsx.rs`:
`parse_inline_str_rich_text_with` (inline `<is><r>` runs) and
`parse_shared_string_rich_text` (`<si><r>` runs from
`xl/sharedStrings.xml`). They are 104 vs 107 LOC, share the same event arms
and identical run-building logic (`current_font.name`/`current_font.size`,
`has_rpr`, `apply_rpr_child`, `RichTextRun`); they differ only in the outer
container element (`<is>` vs `<si>`) and how results are collected (per-cell
vs per-`<si>` index). The run grammar
`<r><rPr>...</rPr><t>...</t></r>` is duplicated across both.

This duplication is the root cause of two issues surfaced in the PR #63 review
of `fix-rich-text-calibri-leak`:

1. The no-`<rFont>` leak fix (seed an empty font) was applied to *both*
   functions, but only the inline path got a regression test. The
   shared-string path's fixed branch is unguarded — a future revert there
   silently reintroduces the reported Calibri leak.
2. The empty-font seed + comment is copied verbatim in both functions.

A single shared run-extraction core fixes the leak in one place, lets one test
guard both paths, and makes "fixed one but not the other" structurally
impossible.

## What Changes

- Extract one shared `collect_runs` core that consumes the run grammar
  (`<r><rPr>...</rPr><t>...</t></r>`) from a `quick_xml` reader positioned at the
  container start, seeded with the empty (null name/size) font, returning
  `Vec<RichTextRun>`.
- `parse_inline_str_rich_text_with` and `parse_shared_string_rich_text` become
  thin wrappers: each walks its outer container (`<is>`/cells, `<si>`), calls
  the shared core per run-container, and collects results (per-cell vs
  per-`<si>` index).
- The empty-font seed (and the no-Calibri invariant) lives in exactly one
  place — the shared core — and the duplicated seed block is deleted from both
  callers.
- Add a shared-string regression test mirroring the inline one, so both paths
  are guarded (closes the #63 review gap).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `rich-text`: clarify that inline and shared-string run parsing share a single
  extraction core, so run behavior (including the no-`<rFont>` → `null`
  invariant) is guaranteed identical across both paths and guarded by one test.
  Delta added under `specs/rich-text/spec.md`.

## Impact

- No public API change. `RichTextRun`, `Font`, `CellValue` unchanged.
- Behavior unchanged for existing callers; the reader still produces identical
  runs.
- Blast radius: refactor of two private functions in `src/reader/xlsx.rs`
  (both used solely by the rich-text read path). The shared core is private.
- Test coverage: net improvement — the shared-string path now has a no-`<rFont>`
  regression test (closes the #63 review gap); the inline test remains.
- Risk: refactor of read-time parsing; mitigated by existing rich-text
  reader / round-trip tests plus the new shared-string test.
