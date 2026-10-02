# 024 — Style write-only in v0.2.0

**Status:** superseded
**Recorded at:** v0.2.0
**Superseded by:** style reading shipped in v0.3.0

## Decision

Ship v0.2.0 with a complete style **writer** and no style **reader**. Reading styles adds
calamine style-table parsing to the reader path, which v0.2.0 would not take on.

## Context

Writing styles requires building a style table (numFmts, fonts, fills, borders, `cellXfs`)
and emitting indices. Reading requires parsing that same table back and resolving each
cell's `s` index. The two are separable, and the writer was the higher-value half for
generated files.

## Alternatives considered

- **Neither in v0.2.0.** Rejected — write-only was chosen to land the common case.
- **Both or nothing.** Rejected — a long gap with no style support at all.

## Why superseded

Style reading shipped in **v0.3.0**, completing the round-trip. The sequencing was
deliberate and short-lived.

## Sources

- `CHANGELOG.md` — v0.2.0 and v0.3.0 style entries.
- `src/reader/styles.rs` — `StyleTableRead`, `resolve_style`, the per-sheet
  cell-to-`cellXfs`-index map.
- `src/writer/styles.rs` — the write side.
- Rationale recovered from the v1.0.0 specification's one-line table entry.
