# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/levu304/excelrs/releases/tag/v0.1.0
