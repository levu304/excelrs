## Why

excelrs silently loses three worksheet-level metadata fields on read→write round-trip:
sheet visibility state (`ws.state`), tab color (`ws.properties.tabColor`), and default
row/column dimensions (`ws.properties.defaultRowHeight` / `defaultColWidth`). The reader
never parses them and the writer never emits them, so reading a workbook with a hidden
sheet and re-writing it makes that sheet **visible again** — a mutating data-loss bug, not
just a missing API. The v2.0.0 release declared the ExcelJS-4.4.0 v1.x drop-in parity
program "complete" while explicitly deferring these three worksheet-level features to a
post-v2.0.0 triage that never happened. They are cheap, single-attribute or
single-element additions and cover common ExcelJS usage patterns.

## What Changes

- Add `ws.state` (`'visible' | 'hidden' | 'veryHidden'`) read and write, mapped to the
  `state` attribute on `<sheet>` in `xl/workbook.xml`.
- Add `ws.properties.tabColor` (a `Color`) read and write, mapped to
  `<sheetPr><tabColor .../></sheetPr>` — the first child of the `<worksheet>` element.
- Add `ws.properties.defaultRowHeight`, `defaultColWidth`, `outlineLevelRow`,
  `outlineLevelCol` read and write, mapped to `<sheetFormatPr .../>` (emitted after
  `<sheetViews>`, before `<cols>`).
- Extend `AddWorksheetOptions` with `state` and `properties` so
  `addWorksheet(name, { state, properties })` mirrors ExcelJS.
- No breaking changes — purely additive surface and round-trip fidelity fixes.

## Capabilities

### New Capabilities

- `worksheet-metadata`: Worksheet-level metadata — sheet visibility state, tab color, and
  default row/column dimensions — preserving them on read→write round-trip and exposing
  them through an ExcelJS-compatible API.

### Modified Capabilities
<!-- none — no existing capability requirements change -->

## Impact

- **Model**: `src/model/worksheet.rs` (new fields + getters/setters), `src/model/workbook.rs`
  (`AddWorksheetOptions` gains `state` + `properties`).
- **Reader**: `src/reader/xlsx.rs` — three new per-sheet parse functions (sheet-state from
  `workbook.xml`, `sheetPr`/`tabColor`, `sheetFormatPr`), attached in the Step 3.x pipeline.
- **Writer**: `src/writer/xlsx.rs` — emit `state` attr on `<sheet>` in `write_workbook_xml`;
  emit `<sheetPr>` before `<dimension>` and `<sheetFormatPr>` between `<sheetViews>` and
  `<cols>` in `write_sheet_xml`.
- **Types**: `index.d.ts` + `dts-header.d.ts` — add `WorksheetState`, `WorksheetProperties`,
  and `tabColor`/`properties` fields; reuse existing `Color`.
- **Tests**: round-trip fixtures (hidden sheet stays hidden; tab color survives; default
  row height survives) plus LibreOffice/XSD conformance checks (element ordering).
- **Out of scope**: the streaming write path (`src/stream.rs`) intentionally emits minimal
  worksheet XML and already documents that per-sheet styling/metadata is out of scope by
  design (v2.0.0 capstone note).
