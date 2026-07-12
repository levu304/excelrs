## Why

excelrs advertises drop-in exceljs compatibility (design principle P1). The
roadmap (docs/spec.md §9.3) lists "Data validation read/write" as a future
capability; nothing ships it yet. Today a `.xlsx` carrying `<dataValidations>`
in `xl/worksheets/sheetN.xml` is read by excelrs with that data silently
dropped, and there is no API to author validations — a correctness gap.

This change closes that item as a single, focused release (matching the
v0.6.0 single-feature shape).

## What Changes

- Add a `DataValidation` model type (`src/model/data_validation.rs`)
- Expose a per-`Worksheet` API: `dataValidations` getter,
  `addDataValidation(dv)`, `getDataValidation(sqref)`,
  `removeDataValidation(sqref)` — range-keyed via `sqref`.
- Writer emits `<dataValidations>` per sheet (after `<hyperlinks>`)
- Reader parses `<dataValidations>` directly from each sheet XML via the zip
  archive (calamine exposes none)
- Supported `type`: `whole`, `decimal`, `list`, `date`, `time`, `textLength`,
  `custom`; all standard operators; `allowBlank`, `showInputMessage`,
  `showErrorMessage`, `errorStyle`, `prompt`, `error` attributes

## Capabilities

### New Capabilities

- `data-validation`: per-worksheet data validation read and write.
  Supported OOXML types and attributes. exceljs-compatible model shape.

### Modified Capabilities

*(none — no existing specs are modified)*

## Impact

- **Code**: new `src/model/data_validation.rs`; `Worksheet` gains 4 methods +
  internal field; `write_sheet_xml` emits block; reader parses + attaches.
- **API**: new `DataValidation` interface + `Worksheet` methods in `index.d.ts`.
  No breaking changes.
- **Dependencies**: none added (reuses `zip`/`quick_xml`).
- **Tests**: Rust unit tests in model + writer/reader; JS vitest round-trip.
