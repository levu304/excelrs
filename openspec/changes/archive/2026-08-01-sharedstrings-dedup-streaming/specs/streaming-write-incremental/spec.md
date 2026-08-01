# streaming-write-incremental Specification

## Purpose

Delta for change `sharedstrings-dedup-streaming`. This is a **change spec
(delta)**, not the canonical capability spec — do not copy to
`openspec/specs/`. Pins the shared-strings interning behavior of the streaming
writer core (`src/stream.rs` `StreamSession`), which the canonical spec at
`openspec/specs/streaming-write-incremental/spec.md` was already supposed to
describe but left "Deferred."

## Status

Unchanged by this delta. The capability remains **Deferred**: the JS-bridge
buffer `src/stream_handle.rs` `StreamWriter.sheets: Vec<StreamSheet>` is still
O(sheets) (ADR-005). This delta pins **only** the interning scenario, which the
code already satisfies — it just was untested.

## MODIFIED Requirements

### Requirement: Streaming writer dedups shared strings across all sheets in a session

The streaming writer SHALL dedup string values via a session-level shared-string
interner: for any string value appearing in one or more cells across one or
more sheets in a single `StreamSession`, `xl/sharedStrings.xml` SHALL contain
that string exactly once (`count` and `uniqueCount` equal the number of distinct
string values), and every cell referencing it SHALL use the same zero-based
shared-string index.

#### Scenario: Duplicate string across sheets maps to one sharedStrings entry

- **WHEN** the same string value is written as a text cell in sheet 1 and in
  sheet 2 via the streaming writer (`stream_write` / `stream_write_to_file`)
- **THEN** `sharedStrings.xml` contains that string exactly once, and both
  sheets' cell nodes (`t="s"`) reference the same `<v>` index

#### Scenario: Duplicate string within a sheet maps to one sharedStrings entry

- **WHEN** the same string value appears in two cells of the same sheet
- **THEN** `sharedStrings.xml` contains that string exactly once and both cells
  reference the same shared-string index
