# 016 — Emit `<dimension ref="..."/>` in sheet XML

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

Every written worksheet includes a `<dimension ref="A1:..."/>` element declaring the used
cell range.

## Context

The element declares the used range so Excel computes correct scroll bounds and print area.
Without it Excel falls back to scanning, and some viewers render the sheet incorrectly.

## Alternatives considered

- **Omit it.** Rejected — it is part of correct sheet XML and affects rendering.
- **Always emit a fixed range** such as `A1:XFD1048576`. Rejected — it defeats the
  declaration's purpose.

## Notes

`<dimension>` must appear in schema order — after `<sheetPr>` and before `<sheetData>`.
This ordering is load-bearing and is annotated in the writer.

## Sources

- `src/writer/xlsx.rs:1699,1712` — emission and the ordering comment.
- Rationale recovered from the v1.0.0 specification's one-line table entry.
