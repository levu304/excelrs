# 001 — Target the ExcelJS API surface

**Status:** current
**Recorded at:** v0.1.0 (June 2026)

## Decision

The TypeScript public API mirrors ExcelJS 4.4.0. Method names, signatures, and the
conceptual model (Workbook → Worksheet → Row → Cell) stay identical, so a user can swap
`require('exceljs')` for `require('@levu304/excelrs')` with minimal code changes.

## Context

ExcelJS has 15K+ GitHub stars and 2K+ forks, but no release in three years. Its pure-JS,
callback-heavy architecture is slow on large workbooks and resists integration with native
tooling. Compatibility is the entire value proposition: a drop-in swap is worth far more to
an existing user than any individual feature ExcelJS lacks.

## Alternatives considered

- **A cleaner, Rust-native API.** Rejected. It would be a better library and a useless
  migration. The audience is defined by what they already have.
- **Bug-for-bug compatibility with ExcelJS defects.** Rejected. Divergences are permitted
  where ExcelJS behavior is a defect, and each must be justified and documented.

## Consequences

Breaking an ExcelJS-compatible behavior is a cost, not a default. It must be earned.

## Sources

- `package.json:3` — package description states the ExcelJS port explicitly.
- `README.md` — migration framing.
- Rationale recovered from the v1.0.0 specification's one-line table entry; fuller
  reasoning not recovered.
