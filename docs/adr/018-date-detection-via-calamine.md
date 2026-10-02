# 018 — Date detection via calamine's built-in format IDs

**Status:** current (corrected — see *Correction*)
**Recorded at:** v0.1.0 (June 2026)

## Decision

Rely on calamine for date detection: its built-in format IDs 14–22 and custom format
strings carrying date tokens. calamine also handles the 1900/1904 date-system
distinction.

## Context

Excel's date system is a workbook-level property, and whether a numeric cell is a date
depends on its number format. Getting this wrong produces silently wrong values, and
reimplementing the format-ID table is exactly the kind of edge-case surface a port should
not take on.

## Alternatives considered

- **A write-path heuristic** matching format strings for `yy`/`mm`/`dd`/`hh`/`ss` tokens.
  Considered and noted in the original spec as a simplification for the write path only.
- **Reimplement the format table.** Rejected — calamine already has it, tested.

## Correction (added at v2.9.1)

**This decision has no excelrs-side implementation.** The text implies excelrs evaluates
format IDs 14–22; it does not. Detection is entirely calamine's, enabled by the `dates`
feature (`Cargo.toml:23`). excelrs only maps the result:

- `Data::DateTime(dt)` → a Date cell value (`src/reader/xlsx.rs:2529`)
- `Data::DateTimeIso(s)` → **a plain string** (`src/reader/xlsx.rs:2534`)

That second mapping is the variant gap the first architecture review raised as P1 —
calamine has `DateTimeIso` and `DurationIso` with no corresponding `CellValue` variant. It
is **still open**, and ISO-8601 date cells read back as strings rather than `Date`
instances.

## Sources

- `Cargo.toml:23` — `calamine = { version = "0.35", features = ["dates"] }`.
- `src/reader/xlsx.rs:2529,2534,3162-3172` — the mapping and its tests.
- `docs/architecture-review.md` — the P1 table listing `DateTimeIso` / `DurationIso` as
  having no excelrs variant.
- `docs/specs/date-cell-value/` — the shipped capability.
- OpenSpec change `reconstruct-design-record`, task 2.2.
