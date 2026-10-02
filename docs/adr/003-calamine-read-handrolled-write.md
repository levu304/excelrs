# 003 — calamine for read, hand-roll for write

**Status:** current (scope corrected — see *Scope correction*)
**Recorded at:** v0.1.0 (June 2026)

## Decision

Use calamine as the XLSX read path. Hand-roll the write path on `zip` + `quick-xml`.

## Context

calamine is battle-tested for reading and handles shared strings, date-system detection,
and malformed files that a hand-rolled reader would not. Writing demands the opposite: full
control over the exact OOXML emitted, because round-trip fidelity and Excel compatibility
are the product. calamine is a reader and gives no leverage over output.

## Alternatives considered

- **calamine for both directions.** Rejected — no control over emitted XML.
- **Hand-roll both.** Rejected — reimplementing shared-string resolution and date-system
  edge cases for no gain.
- **A third-party writer crate.** Rejected — the emitted-XML control requirement is not
  negotiable, and depending on one would inherit its abstractions.

## Scope correction (added at v2.9.1)

This decision originally read as covering *all* reading. It does not. The **streaming**
read and write paths are hand-rolled SAX with no runtime calamine dependency, added in
v2.0.0.

calamine survives in `src/stream.rs` only as a **documented behavioral oracle**: the
shared-formula reference translation is annotated "ported from calamine 0.35.0", and the
`replace_cell_names` unit tests mirror calamine's behavior so the two paths can be compared.
That is a deliberate parity harness, not a dependency.

So the accurate statement is: **calamine backs the in-memory read path; the streaming path
is hand-rolled on both sides and uses calamine only as a reference implementation.**

## Sources

- `src/reader/xlsx.rs` — `open_workbook_auto_from_rs`, `Sheets<R>`, `Data`, `CellErrorType`.
- `src/stream.rs:610-614` — independent `SharedFormulaTable`; calamine noted as port origin.
- `Cargo.toml:23` — `calamine = { version = "0.35", features = ["dates"] }`.
- OpenSpec change `reconstruct-design-record`, task 2.1.
