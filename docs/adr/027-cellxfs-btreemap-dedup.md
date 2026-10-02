# 027 — `BTreeMap` dedup for `cellXfs`, for stable style indices

**Status:** current
**Recorded at:** v0.2.0

## Decision

Deduplicate the style table — numFmts, fonts, fills, borders, and `cellXfs` — using
`BTreeMap` keyed on each style's canonical serialized form.

## Context

Every written cell carries `s="<index>"` into `cellXfs`. If index assignment is
non-deterministic, the same input workbook produces byte-different output across runs, and
round-trip tests cannot compare bytes.

## Alternatives considered

- **`HashMap` dedup.** Rejected. Rust randomizes `HashMap` hashing per process, so index
  assignment varies run to run. This breaks round-trip stability outright.
- **An explicit per-cell index** (pre-allocating an XF slot per cell). Rejected — requires
  a pre-pass allocating slots and produces a `cellXfs` table with one entry per cell.
- **A sorted `Vec<Style>` with binary-search insertion.** Rejected — O(n²) on large style
  sets, which is the case that matters.

## Consequences

The canonical key must be deterministic, which constrains how `Style` is serialized:
`None` fields are omitted, `Some("")` is distinct from `None`, and colors are uppercased
before keying. Those rules are specified in `docs/spec.md` §4 (*Canonical style
serialization*) and referenced from the writer.

There are two implementations that must agree: `build_style_table` (batch) and
`StyleAccumulator` (incremental, single-style-at-a-time, used by the streaming writer). The
latter is documented as producing byte-identical output to the former, and the streaming
writer shares the `SharedString` table and render logic with the in-memory path.

## Sources

- `src/writer/styles.rs:1,71,78,188,226-232` — the dedup, its canonical key, and the
  incremental variant. The ADR is cited in-code at lines 1 and 71.
- `src/writer/xlsx.rs:22` — `BTreeMap` import alongside `BTreeSet`.
- Rationale recovered from the v1.0.0 specification's one-line table entry, which preserved
  the alternatives in unusually complete form.
