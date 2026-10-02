# 017 — Remove `Merge`, `RichText`, `Hyperlink`, `SharedString` from v0.1

**Status:** superseded
**Recorded at:** v0.1.0 (June 2026)
**Superseded by:** all four shipped — RichText v0.12.0, Hyperlink v0.11.0, shared strings
  and merged ranges resolved in v0.5.0

## Decision

Ship v0.1 without `Merge`, `RichText`, `Hyperlink`, or `SharedString` `CellValue` variants.
They are dead variants with no calamine source on the read path and no write support.

## Context

This was the pre-implementation review's sharpest structural finding. calamine's `Data` enum
has 11 variants and excelrs's `CellValue` had 5 variants with **no calamine source** —
`Formula`, `Hyperlink`, `RichText`, and `Merge` simply do not appear in calamine's cell data
stream. calamine resolves shared strings, so `SharedString` would never be observable on
read either.

Keeping them would have produced a type that claimed distinctions the read path could not
populate.

## Alternatives considered

- **Keep the variants, populate them lazily.** Rejected — the read path had no source for
  them, so they would always read as `Null`, which is worse than absence: it is a lie that
  type-checks.
- **Extend calamine.** Rejected — not our crate.

## Why superseded

The premise was correct and time-boxed. Each variant was reintroduced once the
corresponding reader or writer work landed: merged ranges in v0.5.0, hyperlinks in v0.11.0,
rich text in v0.12.0. Shared strings are resolved by calamine on read and deduplicated on
write, so they are an internal representation rather than an observable variant.

The review's underlying caution still holds and is worth keeping: **a variant that the read
path cannot populate is a lie that type-checks.** That is why these were removed rather
than shipped empty.

## Sources

- `docs/architecture-review.md` — the calamine `Data` ↔ `CellValue` mapping table, listing
  all five unsupported variants.
- `CHANGELOG.md` — v0.5.0 (merge), v0.11.0 (hyperlink), v0.12.0 (rich text).
- Rationale otherwise recovered from the v1.0.0 specification's one-line table entry.
