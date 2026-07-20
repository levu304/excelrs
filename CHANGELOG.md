# Changelog
<!-- Release process: tag-driven from main. `git tag -a vX.Y.Z -m "..."` then push the tag. -->

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.0] — 2026-07-20

### Added

- **Streaming Node bridge (#25.1, PR #34).** Constant-memory streaming now bridges to Node's streaming model: `StreamReader` is an async iterable yielding one `JsStreamSheet` at a time, `StreamWriter` accumulates sheets and finalizes to a buffer, and `src/stream-bridge` adapters (`readAsReadable` / `writeToWritable`) bridge to Node `Readable` / `Writable`. Only the current sheet is materialized — constant memory for large files.
- **Shared-formula resolution on the streaming read path (#25.2 / #32).** The streaming reader now resolves shared-formula member cells (`<f t="shared">`) to translated `Formula` text, matching calamine's whole-workbook reader.

### Migration

- Non-breaking: both are additive to the `stream` namespace; `StreamValue::Formula` already existed. No public-API shape change beyond the new streaming bridge methods.

## [2.1.1] — 2026-07-20

### Fixed

- **CHANGELOG history gap.** Backfilled the missing `[2.1.0]` entry (streaming hardening) and added this `[2.1.1]` entry. No code or public-API changes; the published native artifact is identical to `2.1.0`.

## [2.1.0] — 2026-07-19

### Added

- **Streaming hardening (#25.3, PR #27).** Follow-up to the v2.0.0 streaming capstone: closed five residual risks (A1–A5) on the `workbook.stream.xlsx` read/write path. Non-breaking — additive to the `stream` namespace; no public-API shape change.

### Fixed

- None.

### Migration

- Non-breaking: all changes additive to the `stream` namespace; `StreamValue::Formula` already existed. No public-API shape change.

## [2.0.5] — 2026-07-19

### Fixed

- **Platform packages missing `repository` field.** The generated
  `package.json` files for platform-specific packages
  (`@levu304/excelrs-darwin-arm64`, `-linux-x64-gnu`, `-win32-x64-msvc`) lacked
  the `repository` field required by npm 11's auto-attached Sigstore provenance
  attestation. Added `"repository": "https://github.com/levu304/excelrs"` to
  each generated platform manifest.

## [2.0.4] — 2026-07-19 (aborted — platform provenance missing)

### Fixed

- **package.json missing `repository` field.** npm 11+ with OIDC detection
  auto-enables Sigstore provenance attestation (`--provenance` is default when
  OIDC is available). The `repository.url` field is required for provenance
  validation — added `"repository": { "type": "git", "url":
  "https://github.com/levu304/excelrs.git" }`.

## [2.0.3] — 2026-07-19 (aborted — provenance attestation failed)

npm 11 auto-attached provenance but `package.json` lacked `repository` field
for validation. Same binary content as v2.0.0.

### Fixed

- **Release publish auth.** v2.0.1 and v2.0.2 failed because Node 22.14.0 as
  resolved by the pinned `setup-node` SHA ships npm 10.x, which does not
  support OIDC token exchange. Added `npm install -g npm@11` before the
  publish step to ensure npm CLI ≥11.5.1 can detect the
  `ACTIONS_ID_TOKEN_REQUEST_TOKEN` env var and perform the OIDC exchange.

## [2.0.2] — 2026-07-19 (aborted — npm OIDC detection missing)

Setup-node SHA resolved npm 10.x without OIDC support. v2.0.2 was a tag-only
release; same binary content as v2.0.0.

### Fixed

- **Release publish auth.** v2.0.1 failed because `actions/setup-node` with
  `registry-url` creates a temp `.npmrc` via `NPM_CONFIG_USERCONFIG` that
  shadows the npm CLI's OIDC token exchange. Removed `registry-url` from the
  publish job's setup-node step — npm defaults to `registry.npmjs.org`, and
  the OIDC detection finds the `ACTIONS_ID_TOKEN_REQUEST_TOKEN` env var
  without interference from a stale npmrc.

## [2.0.1] — 2026-07-19 (aborted — publish auth failure)

Attempted OIDC migration; `registry-url` in setup-node blocked OIDC detect.
Skipped at the same SHA as 2.0.0 for the main package.

### Changed

- **Published via npm OIDC trusted publishing.** The release pipeline no longer
  uses a long-lived `NPM_TOKEN` secret. `npm publish` authenticates via a
  per-job OIDC token exchange (npm trusted publishers), eliminating the
  standing-exposure risk of a write token in repo secrets.
- **Bumped Node.js to 22.14.0** in CI and release workflows (was 20). Provides
  OIDC-capable npm 11.x and an LTS through April 2027.

### Removed

- `NPM_TOKEN` secret — the four platform packages plus main package all publish
  via OIDC. The repo secret has been deleted.

## [2.0.0] — 2026-07-19

### Added

- **Streaming XLSX (capstone).** `workbook.stream.xlsx.read(buffer)` and `workbook.stream.xlsx.write(sheets)` stream `.xlsx` I/O without materializing the full in-memory `Workbook` model (SAX parse + per-entry zip). The v2.0.0 FFI collects sheet objects into a JS array; constant-memory Node `Readable`/`Writable` / `AsyncIterable` bridging is a deferred follow-up (the Rust core already streams row-by-row). Cell *values* (number / string / boolean / formula) cross the FFI; per-cell styles remain on the in-memory `xlsx` path.
- **ExcelJS-4.4.0 v1.x drop-in parity program declared complete.** Every v1.x targeted area is now `shipped`. Explicitly out of scope (tracked for post-v2.0.0 triage): charts, pivot tables, formula evaluation, themes-write, sheet state (visible/hidden), tab color, default worksheet properties.

### Fixed

- None.

### Migration

- v2.0.0 is non-breaking: streaming is purely additive (new `stream` namespace). All 1.x APIs (`read`/`write`, `csv`, worksheet structure, styling, tables, conditional formatting) are unchanged.

## [1.2.2] — 2026-07-18

### Added

- None.

### Fixed

- Restore merge-cells ranges on the read path (`ws.mergedRanges`).
- Restore row-level style (`<row s="N">`) on the read path (`Row.style`).
- Add `Worksheet.isMerged(row, col)` query to test merged-range membership.
- Tolerate namespace-prefixed `<x:mergeCell>` from non-conformant producers.

## [1.3.0] — 2026-07-18

### Added

- Row/Column `outlineLevel` (0–7) grouping (API, read, write).
- `Worksheet.rowBreaks` / `colBreaks` page break getters/setters (API, read, write).
- `Worksheet.insertRow(rowNumber, values?)` — shift rows below down by one.
- `Worksheet.spliceRows(start, count, rows?)` — remove + insert rows.
- `Worksheet.duplicateRow(rowNumber, count, includeStyle)` — copy rows.

### Fixed

- **RC-1 (Critical):** XML range bomb — cap column range at 16384 in reader.
- **RC-2 (High):** `spliceRows` panic — clamp insert index on start > row count.
- **RC-3 (High):** `duplicateRow` Arc aliasing — `detach_styles()` deep-copy before `clear_styles()`.
- **RC-4 (High):** `insertRow` phantom row — return actual post-renumbered position.

## [1.2.1] — 2026-07-18

### Added

- None.

### Fixed

- **Release hardening (issue #3):**
  - `Worksheet::set_cell_style` now delegates to `Cell::set_style` (single source of truth) instead of re-implementing parse/validate via `set_style_raw`; removed the incorrect `#[napi(setter)]` renames-the-symbol comment.
  - Release `Functional smoke test` now writes a styled workbook, reads it back through the parser, and asserts `font.bold` + `fill.foreground` survive the round-trip — failing the release job on any style loss.

### Changed

- None.

## [1.2.0] — 2026-07-17

### Added

- **Conditional formatting** (ExcelJS `ws.addConditionalFormatting` parity):
  - `Worksheet.addConditionalFormatting(opts)` — append `<conditionalFormatting>` rules with document-order `priority` auto-assignment; covers `cellIs`, `expression`, `colorScale`, `dataBar`, `iconSet`, `top10`, `unique`/`duplicate`, `containsText`, `timePeriod`, blanks/errors/nonBlanks.
  - `Worksheet.getConditionalFormatting()` — return parsed `ConditionalFormat[]` grouped by `sqref`.
  - Rule `style` (font/fill/border/numFmt) is stored as a `dxf` in `<dxfs>` and resolved back on read; foreign (non-CF) `dxfs` are preserved across write.
  - `ConditionalFormat`, `CfRule`, `Cfvo`, `CfColor` JS types.
  - Round-trip read/write of all rule types with priorities and resolved dxf styles preserved.

### Fixed

- None.

### Changed

- None.

## [1.1.0] — 2026-07-16

### Added

- **Tables** (ExcelJS `ws.addTable` parity):
  - `Worksheet.addTable({ name, ref, headerRow, totalsRow, columns, rows, style, autoFilter })` — writes the `xl/tables/tableN.xml` part, registers the sheet relationship, and populates header/data/totals cells.
  - `Worksheet.getTable(name)` / `Worksheet.getTables()` — return parsed `Table` models.
  - `Worksheet.removeTable(name)` — removes the model, XML part, and relationship (leaves underlying cells intact).
  - `Table`, `TableColumn`, `TableRow`, `TableStyle`, and `AddTableOptions` types.
  - Round-trip read/write of structured tables (header row, data rows, totals row, `autoFilter` range, and header style metadata).

### Fixed

- None.

### Changed

- None.

## [1.0.0] — 2026-07-16

### Added

- **Drop-in ExcelJS compatibility milestone.** Full worksheet & workbook parity for the five remaining medium-effort areas (OpenSpec `v1-0-0`):
  - **Headers & footers** — `ws.headerFooter` read/write (`<headerFooter>` with `&C`/`&L`/`&R` format codes).
  - **Page setup / print** — `ws.pageSetup` read/write (`pageMargins`, `paperSize`, `orientation`, `printArea`, `printTitles` via defined names).
  - **Workbook views & calc properties** — `workbook.views` / `workbook.calcProperties` (`<bookViews>`, `<calcPr>`).
  - **Comments** — `Cell.note` / `Cell.comment` read/write (`xl/commentsN.xml` + relationship, authors list).
  - **Images / drawings** — `ws.addImage` read/write (`xl/drawings/`, `xl/media/`, anchors, relationship resolution).

## [0.13.0] — 2026-07-15

### Added

- **Theme-color write (resolved ARGB)** — writer emits `<color rgb="..."/>` with the fully-resolved 8-char ARGB for theme/indexed/rgb sources, so downstream consumers (ExcelJS 4.4.0) round-trip theme colors without needing `theme1.xml` (OpenSpec `theme-color-references`).
- **JS `Date` bridge** — `Cell.date` getter returns a JS `Date` for Date-type cells (`null` otherwise); `Cell.value` setter accepts a raw `Date` and stores it as a date serial (OpenSpec `date-cell-value`).
- **Async contract enforcement** — `read`/`write`/`readFile`/`writeFile` are async; tests enforce `await` (OpenSpec async-contract).

### Fixed

- `Cell::value()` no longer builds a discarded `JsDate` (dead `create_date` + `unsafe` transmute); returns `CellValue` directly. `date()` keeps the live `JsDate` path.
- `set value` JSDoc documents the 3-path dispatch (Date→serial, primitives→variant, objects→`Null`).

### Changed

- Bump `0.12.0 → 0.13.0` (Cargo + npm). **Breaking**: reading a Date cell now returns a `CellValue` with `valueType:"Date"` + `dateSerial` (plus new `Cell.date` getter) instead of a raw ISO string. Documented as a breaking change.

## [0.12.0] — 2026-07-14

### Added

- **RichText read** Parse inline `<is>` strings in worksheet cells (`<c><is><r>`) into `RichTextRun`s; run's `<rPr>` font (name/size/bold/italic/underline/color) mapped via existing `Font` model. Closes rich-text read gap (write shipped earlier).
- **Gradient fill read** Parse `<gradientFill type="linear|path">` and its `<stop position color>` children into `Fill` (`kind="gradient"`, `gradient_type`, `gradient_stops`); theme/indexed/rgb stop colors resolved via `parse_color`.
- **Diagonal border read** Parse `<diagonal>` side plus `diagonalUp`/`diagonalDown` booleans into `Border`.

### Changed

- Bump version 0.11.0 → 0.12.0 (Cargo + npm). No breaking public-API changes.

### Fixed

- Reader hardening PR #14 review: all zip-entry reads bounded 16 MiB (`entry.take(MAX_ENTRY_BYTES)`) prevent zip-bomb OOM; all parse loops carry an event-count cap (`MAX_EVENTS`) halt runaway parses. Applied systemically across xlsx.rs, styles.rs, workbook.rs.

### Notes

- JS `Date` bridge shipped in v0.13.0 (see [0.13.0] above).
- Publishes `@levu304/excelrs` + platform packages (`-darwin-arm64`, `-linux-x64-gnu`, `-win32-x64-msvc`) via `v0.12.0` tag → release.yml.

## [0.10.0] — 2026-07-13

### Added

- **ExcelJS porting roadmap** — `ROADMAP.md` at repo root: full ExcelJS (pinned 4.4.0) → excelrs parity matrix (30 feature areas, status `shipped`/`partial`/`planned`/`n-a`) plus a prioritized porting roadmap sequenced across v0.11.0 → post-v1.
- **`exceljs-parity` OpenSpec capability** — tracks parity and governs how the porting roadmap is derived, prioritized, and consumed by future releases.

### Changed

- Synced `theme-color-references` and `indexed-color-references` specs to `openspec/specs/` (shipped in v0.6.0; previously unsynced).
- Archived OpenSpec changes `v0-10-0-exceljs-roadmap-align` and `v0.6.0-theme-color-references`.

### Notes

- Docs-only release. No runtime or public-API changes. `package.json` intentionally **not** bumped (stays `0.9.0`); no npm publish. Tag `v0.10.0` is a roadmap milestone marker only.

## [0.11.0] — 2026-07-14

### Added

- **Hyperlinks (read)** — Parse `<hyperlinks>` from sheet XML and resolve `r:id` via `xl/worksheets/_rels/sheetN.xml.rels`. `CellValue` hyperlink/`hyperlink_text` round-trip is now closed (write shipped v0.5.0).
- **Auto filters** — Read and write `<autoFilter ref>` attribute on worksheets. Exposed as `ws.autoFilter`.
- **Freeze panes / split views** — Read and write `<sheetViews><sheetView><pane>` state. Exposed as `ws.views` array of `SheetView` descriptors.
- **Sheet protection** — Read and write `<sheetProtection>` boolean flags. Exposed as `ws.protection`.

### Changed

- Parity matrix advances `hyperlinks` (read), `auto-filter`, freeze panes, and sheet protection from `planned` to `shipped`.
- Bumped `excelrs` to `0.11.0` (npm) / `0.11.0` (Cargo).

## [0.9.0] — 2026-07-13

### Added

- **CSV read/write** — read and write RFC 4180 CSV via a `WorkbookCsv` async
  handle obtained through `wb.csv`:
  - `csv.read(buf)` / `csv.readFile(path)` — parse CSV into a single "Sheet1"
    worksheet; numeric inference on read (f64-parsable fields → Number cells);
    optional `delimiter` parameter (default `,`).
  - `csv.write()` / `csv.writeFile(path)` — serialize the **first** worksheet
    to CSV (CSV is single-sheet); formula cells emit their cached value;
    optional `delimiter` (default `,`) and `withBom` (default `false`).
  - Manual RFC 4180 parser/serializer; no new dependencies.
  - See `docs/spec.md` §9.2.3 for the full capability description.

### Changed

- Bump version 0.8.2 → 0.9.0. No breaking public-API changes.
- docs/spec.md §9.3: "CSV read/write" removed from future list,
  recorded as shipped in new §9.2.3.

## [0.8.2] — 2026-07-12

### Fixed

- Added tsconfig.json and @types/node to resolve TypeScript type errors in tests and type defs
- Fixed stale camelCase/snake_case property mismatches in test files (gradientType, numFmt, wrapText, etc.)
- Fixed null-safety and cast issues in test assertions
- Added tsc --noEmit typecheck script

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

## [0.8.1] — 2026-07-12

### Changed

- Bump version 0.7.0 → 0.8.1. No breaking public-API changes.

### Fixed

- Fixed data validation element order in writer (now emits before `<hyperlinks>` per ECMA-376 schema requirement)
- Fixed OOXML boolean parsing (`true`/`false` → Rust bool) and CDATA formula handling in reader
- Fixed writer to emit explicit `allowBlank="0"` when disabled in `DataValidation`

## [0.8.0] — 2026-07-12 (superseded by 0.8.1)

### Added

- **Data validation** — per-worksheet data validation via `DataValidation` model:
  - `ws.dataValidations` getter returning all validations on the sheet.
  - `ws.addDataValidation(dv)` — add or upsert a validation by `sqref`.
  - `ws.getDataValidation(sqref)` — look up by range; returns `null` when absent.
  - `ws.removeDataValidation(sqref)` — remove by range; no-op if absent.
- Writer emits `<dataValidations>` per sheet (before `<hyperlinks>`, schema-required order) with correct
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

[0.10.0]: https://github.com/levu304/excelrs/releases/tag/v0.10.0
[0.9.0]: https://github.com/levu304/excelrs/releases/tag/v0.9.0
[0.8.0]: https://github.com/levu304/excelrs/releases/tag/v0.8.0
[0.8.1]: https://github.com/levu304/excelrs/releases/tag/v0.8.1
[0.7.0]: https://github.com/levu304/excelrs/releases/tag/v0.7.0
[0.6.0]: https://github.com/levu304/excelrs/releases/tag/v0.6.0
[0.5.0]: https://github.com/levu304/excelrs/releases/tag/v0.5.0
[0.3.0]: https://github.com/levu304/excelrs/releases/tag/v0.3.0
[0.2.2]: https://github.com/levu304/excelrs/releases/tag/v0.2.2
[0.2.0]: https://github.com/levu304/excelrs/releases/tag/v0.2.0
[0.1.0]: https://github.com/levu304/excelrs/releases/tag/v0.1.0
