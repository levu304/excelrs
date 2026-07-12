# Changelog
<!-- Release process: tag-driven from main. `git tag -a vX.Y.Z -m "..."` then push the tag. -->

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] — 2026-07-11

### Added

- **Defined names (named ranges)** — read and write workbook-global and sheet-scoped (local) names via the napi API:
  - `wb.addDefinedName(name, value, sheet?)` — add or upsert a name (sheet-scoped when `sheet` is given)
  - `wb.removeDefinedName(name, sheet?)` — remove a name (no-op if absent)
  - `wb.getDefinedName(name, sheet?)` — look up a name; returns `null` when not found
  - `wb.definedNames` — getter returning all `DefinedName` objects
- Reader parses `<definedNames>` from `xl/workbook.xml`, resolving `localSheetId` → sheet name; tolerant of namespace-prefixed elements and out-of-range IDs
- Writer emits `<definedNames>` with correct `localSheetId` scoping; errors on unresolved sheet scope (prevents silent scope loss)

### Changed

- Bump version 0.6.0 → 0.7.0. No breaking public-API changes.

## [0.8.0] — 2026-07-12

### Added

- **Data validation** — per-worksheet data validation via `DataValidation` model:
  - `ws.dataValidations` getter returning all validations on the sheet.
  - `ws.addDataValidation(dv)` — add or upsert a validation by `sqref`.
  - `ws.getDataValidation(sqref)` — look up by range; returns `null` when absent.
  - `ws.removeDataValidation(sqref)` — remove by range; no-op if absent.
- Writer emits `<dataValidations>` per sheet (after `<hyperlinks>`) with correct
  `type`, `operator`, `sqref`, `<formula1>`/`<formula2>`, and boolean flags.
- Reader parses `<dataValidations>` from each sheet XML via the zip archive,
  attaching parsed validations to the corresponding `Worksheet`.
- Supported types: `whole`, `decimal`, `list`, `date`, `time`, `textLength`,
  `custom`. Full operator set; `allowBlank`, `showInputMessage`,
  `showErrorMessage`, `errorStyle`, `prompt`, and `error` attributes.

### Changed

- Bump version 0.7.0 → 0.8.0. No breaking public-API changes.
- docs/spec.md §9.3: "Data validation read/write" removed from future list,
  recorded as shipped in new §9.2.2.

## [0.6.0] — 2026-07-11

### Added

- Theme color references (`theme="N"`) resolved to ARGB on read via `xl/theme/theme1.xml` color scheme
- Indexed color references (`indexed="N"`) resolved to ARGB on read via the standard 56-entry system palette
- Tint support for theme colors (`tint` attribute applied to theme color resolution)
- Custom theme1.xml parsing with fallback to OOXML default scheme

## [0.5.0] — 2026-07-10

### Added

- Row-level styles (height, hidden, style index in `<row>` element)
- Gradient fills with linear (`degree`) and path (`left`/`right`/`top`/`bottom` geometry)
- Cell style object model with validation
- Diagonal borders (up, down)
- Merged cells (`mergeCells` in sheet XML)
- Hyperlinks (per-sheet `.rels` + `<hyperlinks>` block)
- Rich-text inline formatting (`<is>`/`<r>`/`<rPr>`)

### Fixed

- Row.style silent drop (interior mutability via `Arc<Mutex<>>`)
- Gradient stops validation (< 2 stops rejected)
- `gradientFill` invalid `angle` attribute omitted; correct path geometry emitted
- RichText font.color XML injection (escaped + validated)
- Gradient stop, font color, fill fg/bg, border color sinks escaped

### Changed

- Minimum Rust edition bumped implicitly via dependency updates
- Color values validated to 6/8 hex characters
- All XML `rgb=` attributes escaped for defense-in-depth

## [0.3.0] — 2026-06-30

### Added

- **Style read** — `xl/styles.xml` is now parsed on read via `src/reader/styles.rs`.
  Font, Fill, Border, Alignment, and numFmt are resolved to model `Style` objects
  and attached to each `Cell`. Round-trip of a styled `.xlsx` preserves styles
  end-to-end (v0.3.0 scope, previously `style: None` on every cell). 7 new Rust
  unit tests for the parser; 4 new JS round-trip tests (F16–F18).
- **Alignment emission (writer)** — `<alignment>` child elements in `cellXfs` are
  now emitted for Font, Fill, Border, and numFmt-aligned cells. The `applyAlignment`
  flag is set when `alignment_id != 0`. The vertical "middle" → OOXML "center"
  mapping is handled in the emit function. 3 new Rust tests for dedup/emit/mapping.
- **Style read architecture:** 3-pass reader — calamine for values/formulas, zip
  archive for `xl/styles.xml` and per-sheet `s="N"` attributes, merged at
  cell-creation time. cellStyleXfs inheritance is deferred (v0.3.0 uses cellXf
  directly); theme colors and gradient fills are silently skipped.
- 146 Rust tests (was 127, +15 in PR #2 review follow-up) + 60 JS tests (was 57) = **206 total**.

### Changed

- `Worksheet::set_cell_style` now uses the raw style setter (`set_style_raw`)
  instead of the `#[napi(setter)]` method, which was unreachable from Rust code.
  (Napi-rs generates wrapper code for `#[napi(setter)]` that doesn't dispatch
  when called as a Rust method.)
- `docs/spec.md` §9.2.1: Removed "Style *read*" and "Alignment emission (writer)"
  rows from the deferred-items table. Updated §1 to v0.3.0 scope. Added
  vertical middle→center mapping note to §6.8.

### Fixed

- **Built-in numFmt IDs 0-49 now resolve to format codes** — `resolve_style`
  matches `numFmtId < 50` against a `BUILTIN_NUMFMTS` const table (~19 entries
  for date, percentage, currency, etc.) before falling through to custom IDs.
  Previously all IDs < 50 silently resolved to `None`. (PR #2 review.)
- **applyX flags now honored** — `<xf>` attributes `applyFont`, `applyFill`,
  `applyBorder`, `applyAlignment`, and `applyNumberFormat` are parsed and gate
  sub-field application in `resolve_style`. Previously only the `xf_index != 0`
  check was used, causing third-party files with `applyX="0"` to incorrectly
  apply sub-fields. (PR #2 review.)
- **Module doc rewritten** — `src/reader/styles.rs` module doc now accurately
  reflects that applyX flags are parsed and respected. (PR #2 review.)

## [0.2.2] — 2026-06-30

### Fixed

- **Release pipeline now publishes platform-specific `.node` packages** — the CI
  release workflow only published the JS wrapper; the 3 platform packages
  (`darwin-arm64`, `linux-x64-gnu`, `win32-x64-msvc`) were created but never
  pushed to npm. Fresh `npm install` would fail at runtime with a
  native-binding error. Worked locally because `native.js` loads from the
  repo root first.
- **`optionalDependencies` injected at publish time** — the main package now
  declares the platform packages as optional dependencies so npm installs
  them automatically on the matching platform.
- **GitHub Release auto-created** via `softprops/action-gh-release@v2`.
- **Functional smoke test** runs after publish in CI: fresh install + round-trip.
- **Verify step retries** on npm registry propagation delay.

## [0.2.1] — 2026-06-30 (unpublished — CI pipeline fix)

v0.2.0's release pipeline work was split into v0.2.1 → v0.2.2 when npm
re-publish of the same version was blocked after unpublish. v0.2.2 is the
first fully working release; v0.2.0 and v0.2.1 are superseded.

## [0.2.0] — 2026-06-30 (unpublished — Style System scope)

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

[0.8.0]: https://github.com/levu304/excelrs/releases/tag/v0.8.0
[0.7.0]: https://github.com/levu304/excelrs/releases/tag/v0.7.0
[0.6.0]: https://github.com/levu304/excelrs/releases/tag/v0.6.0
[0.5.0]: https://github.com/levu304/excelrs/releases/tag/v0.5.0
[0.3.0]: https://github.com/levu304/excelrs/releases/tag/v0.3.0
[0.2.2]: https://github.com/levu304/excelrs/releases/tag/v0.2.2
[0.2.0]: https://github.com/levu304/excelrs/releases/tag/v0.2.0
[0.1.0]: https://github.com/levu304/excelrs/releases/tag/v0.1.0
