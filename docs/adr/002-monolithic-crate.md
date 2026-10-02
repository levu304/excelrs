# 002 — Monolithic crate

**Status:** superseded (by ADR-002's own trigger, re-evaluated below)
**Recorded at:** v0.1.0 (June 2026)
**Superseded by:** the trigger stated in *Re-evaluation* below

## Decision

Ship as a single crate rather than a Cargo workspace. Revisit at roughly 5,000 lines of
code.

## Context

At v0.1.0 the crate was a few hundred lines. A workspace split adds manifest, feature-flag,
and cross-crate version-management overhead before any of it buys anything.

## Alternatives considered

- **Workspace split from the start.** Rejected as premature, correctly.

## Re-evaluation

**The trigger fired and was not actioned.** The crate reached ~5K LoC during v0.5.0 and is
now **26,258 LoC across 32 files** — roughly 5x past the point where this decision said to
look again. Nobody looked.

**On examination, crate splitting was never the right lever, and this decision is
superseded rather than implemented.** The module boundaries a split would have created
already exist within the single crate:

| Module | Files | Role |
| --- | --- | --- |
| `model/` | 20 | in-memory types; single source of truth for read and write |
| `reader/` | 4 | calamine-backed in-memory XLSX + style parsing |
| `writer/` | 3 | OOXML emission |
| `formula/` | 3 | opt-in formula evaluation |
| `xlsx/` | 2 | lock-free workbook handle |
| (root) | 6 | crate surface, errors, types |

The concentration that actually warrants action is **file-level, not crate-level**:
`src/model/worksheet.rs` is 1,933 lines against a ~820-line crate average, and
`src/reader/xlsx.rs` is 4,565. Those are the decomposition targets.

The original trigger was well-calibrated for the codebase shape of its day — a small crate
where crate split and module split were the same lever. That equivalence no longer holds,
which is why the trigger is being replaced rather than simply honoured.

## Superseding trigger

**No source file exceeds ~2,000 lines.** This bounds the thing that actually degrades —
merge conflicts, review burden, and compile times — without imposing workspace ceremony the
module layout does not need.

## Note

This is a finding, not a refactor. No code changes accompany it. `worksheet.rs` and
`reader/xlsx.rs` remain over the threshold and are known debt.

## Sources

- Recovered from the v1.0.0 specification's one-line table entry; fuller reasoning not
  recovered.
- File counts and line counts measured directly from `src/` at v2.9.1.
- OpenSpec change `reconstruct-design-record`, design D5.
