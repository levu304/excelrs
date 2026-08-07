## Context

`src/reader/xlsx.rs` has two private functions that parse rich-text runs:
`parse_inline_str_rich_text_with` (inline `<is><r>`) and
`parse_shared_string_rich_text` (`<si><r>`). See proposal.md — Why. They share
an identical run grammar (`<r><rPr>...</rPr><t>...</t></r>`) and run-building
logic; they differ only in the outer container element (`<is>` vs `<si>`) and
result collection (per-cell `Vec<(row,col,runs)>` vs
`HashMap<u32, Vec<RichTextRun>>` keyed by `<si>` index). The two were split
when shared-string reading was added (commit 1f55235), each seeded with an
empty font at run start.

## Goals / Non-Goals

- Goal: one shared run-extraction core; the empty-font seed and no-Calibri
  invariant live in a single place; one regression test guards both paths.
- Non-Goal: do not change the outer worksheet / `<si>` walk structure beyond
  what the extraction needs; do not change writer behavior; do not change
  emitted run semantics or public types.

## Decisions

1. **Extract `fn collect_runs<R: Read>(reader, scheme, end_tag) -> Vec<RichTextRun>`**
   that consumes events from a `quick_xml` reader already positioned at the
   container start, building runs until it sees the matching container end tag
   (`</is>` or `</si>`). Seed the empty font (name/size `None`) here, and carry
   the existing `events > max_events` guard so an unbounded loop is impossible.
   - Alternative: pass a closure stop-condition instead of an `end_tag` — rejected;
     two concrete containers means an explicit end tag is simpler and self-documenting.
   - Alternative: fully unify both outer walks into one function — rejected; the
     per-cell vs per-`<si>`-index collection differs enough that unifying it adds
     more branching than it removes. Keep the split at the *outer* walk, unify
     only the *run* body.
2. **Callers seed nothing.** Both `parse_inline_str_rich_text_with` and
   `parse_shared_string_rich_text` delete their duplicate `current_font =
   Font::default(); current_font.name = None; current_font.size = None;` block
   and the matching comment, calling `collect_runs` once per container.
3. **One test per path.** Keep `test_parse_inline_str_rich_text_no_rfont_no_calibri_leak`
   and add `test_parse_shared_string_rich_text_no_rfont_no_calibri_leak`
   mirroring it (a run with `<b/>`/`<sz>` but no `<rFont>` → `font.name === null`).
   Both go through `collect_runs`, so the invariant is proven on both call paths.

## Risks / Trade-offs

- [quick_xml reader ownership] `collect_runs` borrows the reader mutably for the
  duration. Both callers already own their reader, so `&mut reader` is fine.
- [event cap] The shared core must honor `max_events` like the originals to
  avoid unbounded loops → keep the `events > max_events` break inside
  `collect_runs`.
- [regression] Refactor of read-time parsing → mitigated by existing
  `test_rich_text_roundtrip`, `test_overlay_shared_string_rich_text_replaces_string`,
  and the new shared-string no-`<rFont>` test.

## Migration Plan

N/A — internal refactor; no public API, no stored state, no serialization change.

## Open Questions

- None.
