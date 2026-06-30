# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-06-30

### Added

- **Style system (write only)** — Font, Fill, Border, Alignment, and inline `num_fmt: string`
  on cells and columns. Full style table emitted in `xl/styles.xml` via `BTreeMap`-backed
  dedup of `numFmts`, `fonts`, `fills`, `borders`, and `cellXfs` (spec v1.3.0, ADR-24–27).
  2,294 lines added across 18 files (src/writer/styles.rs: 716 lines — the largest single
  file; src/model/style.rs: 603 lines; 14 new JS integration tests).
- `cell.style = {...}` — getter/setter with full-replace semantics (§6.9). Validates
  ARGB/RGB hex, float finiteness, and enum values (Fill.kind, BorderStyle.style).
- `column.style = {...}` — column-level default style (§6.9). Cells in a column without
  an explicit `cell.style` inherit the column's style at write time.
- `Worksheet.setColumns(cols)` — bulk set column definitions + styles from JS.
  Columns use `Arc<Mutex<Vec<Column>>>` for interior mutability (shared across clones).
- `Worksheet.setCellStyle(row, col, style)` — bypasses clone-on-read to set a cell's
  style through the locked row map.
- `s="<idx>"` attribute on every written `<c>` element (§4.3 step 6). Normal is always
  seeded at index 0. The `s` attribute is never omitted.
- 56 JS integration tests (42 v0.1.0 baseline + 14 new) — round-trip verified against
  exceljs v4.4.0 as the reference reader.

### Fixed

- `CHANGELOG.md` v0.1.0 entry incorrectly stated "4 targets (macOS ARM64/Intel, Linux x64, Windows x64)". The actual release.yml matrix is **3 targets** (macOS ARM64, Linux x64, Windows x64). The `x86_64-apple-darwin` (Intel macOS) target was dropped during release prep when the `macos-13` runner hung. Historical release note is left intact for record; the spec at the time matched the build configuration.
- Writer emitted `<fgColor>` / `<bgColor>` as siblings of `<patternFill>` instead of children. Exceljs silently ignored fills with this OOXML-invalid structure.
- Writer emitted `argb` as the color attribute; OOXML uses `rgb`. Exceljs reads `rgb`.
- `cellXfs` entries were missing `applyFont`/`applyFill`/`applyBorder` flags. Without these, exceljs ignores the referenced sub-table indices.
- Writer applied the wrong column's style to cells when `setColumns` was used with
  sparse column definitions (e.g. defining only column B caused A1 to inherit
  B's style). Fixed by adding `col_num: u32` to `Column` and looking up by column
  number. Sparse usage requires the new `colNum` field; contiguous A/B/C usage
  is unchanged. C14 added.
- `si.next().expect(...)` replaced with typed `ExcelrsError::Write` on
  style-indices exhaustion. The writer now surfaces this internal invariant
  failure as a normal error instead of crashing the process. Unit test added.

### Changed

- **Alignment emission deferred to v0.3.0** (spec §9.2.1). The `alignment` field is
  accepted in the `Style` JS object with full validation, but is silently dropped at
  write time — `<alignment>` child emission in `cellXf` requires non-trivial layout
  work and is bundled with the broader style-read v0.3.0 scope.
- Spec v1.3.3 (post-architect-reviewer pass-2): `num_fmt: Some("")` is rejected with
  `ExcelrsError::InvalidStyle`; common-pitfall callout added to §6.9; §4.3 step 6
  now states explicitly that every written `<c>` carries `s="<idx>"`; §9.2 test budget
  broken down per task; §9.2 notes that the v0.2.0 README update is part of this
  release. No code-affecting changes.
- Spec referred to the npm package as `excelrs` (unscoped). The published v0.1.0
  artifact is `@levu304/excelrs` (scoped). spec.md has been updated everywhere.
- Columns use `Arc<Mutex<Vec<Column>>>` for interior mutability (matching the existing
  `Arc<Mutex<BTreeMap<u32, Row>>>` pattern for rows).

### Notes

- v0.2.0 ships with **3 platform targets**: macOS ARM64, Linux x64, Windows x64.
  The `x86_64-apple-darwin` (Intel macOS) target was dropped in v0.1.0 release prep.
- See spec §9.2 for the full v0.3.0 deferred items list and §9.2.1 for rationale.

### Security

- Rotated the `NPM_TOKEN` GitHub secret after two legacy publish tokens were inadvertently
  exposed in the project's chat history. The replacement token has the same scopes (publish
  to `@levu304/*`, 2FA-bypass). No release was published between exposure and rotation.

## [0.1.0] — 2026-06-29

### Added

- **XLSX reader** — read `.xlsx` files via calamine into the Rust model layer.
  Supports Number, String, Boolean, DateTime, Error, and Formula cells.
  Two-pass algorithm: data pass (cell values) + formula pass (separate calamine API).
- **XLSX writer** — write `.xlsx` files via `zip` + `quick-xml`.
  Supports shared string deduplication, dimension reporting, and Normal-only styles.
- **Model layer** — Workbook, Worksheet, Row, Cell, Column, CellValue with
  exceljs-compatible API surface exposed via napi-rs.
- **Async I/O** — `WorkbookXlsx.read`, `readFile`, `write`, `writeFile` backed by
  napi-rs async runtime.
- **JS glue** — `index.js` with method overload dispatch for `getCell`,
  preserving `index.js` across builds via `--js native.js`.
- **Interior mutability on Worksheet** — `Arc<Mutex<BTreeMap>>` for rows so
  `ws.addRow([...])` works from JS through any cloned reference.
- **Tests** — 73 Rust unit tests + 42 JS integration tests (vitest).
  Round-trip verified with exceljs v4.4.0.
- **CI/CD pipeline** — GitHub Actions matrix across 4 targets
  (macOS ARM64/Intel, Linux x64, Windows x64), plus automated npm publish
  and GitHub Release on tag push.
- **Documentation** — complete spec (docs/spec.md), two architecture reviews.

[0.2.0]: https://github.com/levu304/excelrs/releases/tag/v0.2.0
[0.1.0]: https://github.com/levu304/excelrs/releases/tag/v0.1.0
