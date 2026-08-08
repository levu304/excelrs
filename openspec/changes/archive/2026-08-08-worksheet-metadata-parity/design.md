## Context

excelrs is a Rust→Node native XLSX library that aims to be an ExcelJS drop-in. The reader
(`src/reader/xlsx.rs`) follows a per-sheet parse pipeline: `workbook_inner_from_bytes` runs
~20 independent `parse_sheet_*` functions (each reads the zip bytes, extracts one concern,
and attaches it to `inner.worksheets[i]`). The writer (`src/writer/xlsx.rs`) emits
`xl/workbook.xml` (`write_workbook_xml`, which currently writes bare
`<sheet name sheetId r:id/>`) and each `xl/worksheets/sheetN.xml` (`write_sheet_xml`, whose
order is `<dimension>` → `<sheetViews>` → `<cols>` → `<sheetData>`).

Today none of sheet visibility state, tab color, or default dimensions are parsed or
emitted. Calamine reads all sheets regardless of `state`, so the data is available but
dropped. The gaps are three cheap, well-isolated additions. See proposal.md — Why.

## Goals / Non-Goals

**Goals:**

- Preserve the three metadata fields across read→write round-trip (no more mutating
  hidden→visible behavior).
- Expose them through an ExcelJS-compatible API surface.
- Keep reader/writer changes isolated and schema-order-correct.

**Non-Goals:**

- `showGridLines` — already handled in `emit_sheet_views`.
- Worksheet-level `outlineProperties` (`summaryBelow`/`summaryRight`) beyond the
  `outlineLevelRow`/`outlineLevelCol` shortcuts.
- `dyDescent` — deferred (see Open Questions).
- Streaming write path (`src/stream.rs`) — intentionally value-only; out of scope.

## Decisions

**D1 — napi-rs getter/setter pairs for mutable props.**
ExcelJS uses plain assignment (`ws.state = 'hidden'`). napi-rs objects return clones
across the FFI, so a sub-object `ws.properties.defaultRowHeight` cannot be mutated in place.
Mirror the repo's existing pattern (`views()` at worksheet.rs:768 with paired
`getter`/`setter`): expose `state` via `#[napi(getter)] fn state()` +
`#[napi(setter)] fn set_state()`, and `tabColor` / properties fields the same way. The
`ws.properties` getter returns a read-only `WorksheetProperties` snapshot; a
`setProperties(Partial<WorksheetProperties>)` method handles bulk writes. This keeps reads
fully faithful (the round-trip case) and provides ergonomic setters for writes.

**D2 — Store fields on the `Worksheet` model, not a nested struct.**
Add `state: SheetState`, `tab_color: Option<Color>`, `default_row_height: Option<f64>`,
`default_col_width: Option<f64>`, `outline_level_row: Option<u8>`,
`outline_level_col: Option<u8>` directly to `Worksheet`. Reuses the existing `Color` model
(`src/model/color.rs`) for tab color, so theme/indexed/ARGB variants come for free.

**D3 — `SheetState` as a string-backed enum.**
Mirror ExcelJS `WorksheetState = 'visible' | 'hidden' | 'veryHidden'`. Serialize to the
OOXML `state` attribute verbatim; omit the attribute when `visible` (default) to match
ExcelJS output and keep diffs minimal.

**D4 — Reader as three new `parse_sheet_*` functions in the existing pipeline.**

- `parse_sheet_states(data)` walks `xl/workbook.xml` `<sheet>` elements and captures the
  `state` attribute, attached by index (the sheet ordering is already stable in the
  pipeline).
- `parse_sheet_tab_colors(data, sheet_count)` walks each `xl/worksheets/sheetN.xml` for
  `<sheetPr><tabColor>` and resolves it via the existing `Color` parser.
- `parse_sheet_format_pr(data, sheet_count)` walks `<sheetFormatPr>` for
  `defaultRowHeight`/`defaultColWidth`/`outlineLevelRow`/`outlineLevelCol`.
Each hooked into `workbook_inner_from_bytes` as a new Step 3.x, consistent with neighbors.

**D5 — Writer element placement respects CT_Worksheet ordering.**
CT_Worksheet child order is `sheetPr` → `dimension` → `sheetViews` → `sheetFormatPr` →
`<cols>` → `<sheetData>`. `write_sheet_xml` currently emits `dimension` first, so:

- emit `<sheetPr>` (with `<tabColor>`) **before** `<dimension>`,
- emit `<sheetFormatPr>` **between** `<sheetViews>` and `<cols>`.
`write_workbook_xml` adds the `state` attribute to the existing `<sheet>` template.

**D6 — Add to `AddWorksheetOptions`, not a new type.**
`src/model/workbook.rs` already mirrors ExcelJS `AddWorksheetOptions` with `page_setup`,
`views`, etc. Add `state: Option<SheetState>` and `properties: Option<WorksheetProperties>`
and apply them in `add_worksheet`. `index.d.ts` gains `WorksheetState`,
`WorksheetProperties`, and the two new option fields.

## Risks / Trade-offs

- **[Element-order strictness]** → Excel/LibreOffice are lenient but the repo has
  golden-file + XSD/LibreOffice conformance tests; placing `sheetPr`/`sheetFormatPr`
  correctly (D5) and adding a conformance fixture keeps it green.
- **[`state` on hidden sheets + calamine]** → Calamine loads all sheets regardless of
  `state`, so no read-side filtering change is needed; the attribute is purely a write
  concern. Verified.
- **[`x14ac:dyDescent` namespace]** → Skipped from this change (D2 non-goal); an xlsx
  without `dyDescent` still opens fine in Excel. Trade-off: that one attribute is not yet
  preserved. Acceptable — see Open Questions.
- **[Two sources of `outlineLevel`]** → `sheetFormatPr` `outlineLevelRow/Col` (sheet-level
  default) is distinct from the already-shipped per-row/column `outlineLevel` attribute.
  Keep them separate; do not conflate.

## Migration Plan

Purely additive (new getters/setters + new option fields). No breaking API or file-format
change. Ship as a minor version; no rollback special-casing beyond a normal release.

## Open Questions

- **`dyDescent`**: include it (requires adding the `x14ac` namespace declaration to the
  `<worksheet>` root element) or leave non-preserved for now? Low user impact; defaulting
  to "leave out" keeps the change minimal. Resolve at implementation if a conformance check
  demands it.
- **Flat `ws.tabColor` sugar**: ExcelJS only exposes `ws.properties.tabColor`, but the repo
  convention favors flat getters (e.g. `ws.rowBreaks`). Adding a flat `ws.tabColor`
  getter/setter is optional sugar; default to keeping it under `ws.properties` only unless
  a user requests parity for the legacy shape.
