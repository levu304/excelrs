# Proposal: v1.2.0 — Conditional formatting

## Why

`excelrs` reached the v1.1.0 tables milestone, but the parity matrix still lists
**Conditional formatting** as `targeted` for v1.2.0. ExcelJS users expect
`ws.addConditionalFormatting({ ref, rules })` / `ws.getConditionalFormatting()`
with the full rule-type set (`cellIs`, `expression`, `colorScale`, `dataBar`,
`iconSet`, `top10`, `unique`/`duplicate`, `containsText`, `timePeriod`,
blanks/errors/nonBlanks) and priority ordering. Today those calls are missing,
so any workbook relying on conditional formats fails or round-trips them as
plain cells — breaking the compat promise for a core spreadsheet feature.

Conditional formatting also exercises a second OOXML surface that `excelrs`
does **not** yet touch: the **`dxfs`** (differential formats) collection in
`xl/styles.xml`. The styles reader currently **skips** `dxfs`
(`src/reader/styles.rs:577`) and the writer does not emit it, so a real
conditional-format workbook cannot round-trip today. v1.2.0 closes both gaps.

## What Changes

- Add **Conditional formatting** as a first-class worksheet feature with full
  read/write round-trip:
  - `ws.addConditionalFormatting({ ref, rules })` — attach one or more rules to a
    cell range; `ws.getConditionalFormatting()` — retrieve all rules grouped by range.
  - `CfRule` covers every roadmap rule type: `cellIs`, `expression`, `colorScale`,
    `dataBar`, `iconSet`, `top10`, `unique`, `duplicate`, `containsText`,
    `timePeriod`, `containsBlanks`/`notContainsBlanks`, `containsErrors`/`notContainsErrors`.
  - Worksheet-global, unique `priority` ordering (mirrors ExcelJS array order;
    preserved on read).
  - `colorScale` / `dataBar` / `iconSet` carry inline `cfvo` (conditional-format
    value objects) + colors; all other rule types carry a `style` (font/fill/border).
- Implement **`dxfs` read + write** in `xl/styles.xml` (currently skipped). A rule
  with a `style` is stored as a differential format and referenced by `dxfId`;
  `colorScale`/`dataBar`/`iconSet` need no `dxfId` (their visuals are inline).
- Emit/parse the worksheet `<conditionalFormatting sqref="…">` element (after
  `<sheetData>`, in schema-correct position) and the styles `<dxfs>` element
  (after `cellXfs`, before `tableStyles`).
- **BREAKING**: none — strictly additive. Minor version bump
  (`1.1.0` → `1.2.0`) signals new capability; no existing API behavior changes.

## Capabilities

### New Capabilities

- `conditional-formatting`: Worksheet conditional formats — `ws.addConditionalFormatting`
  / `ws.getConditionalFormatting`, the `ConditionalFormat` / `CfRule` model, full
  rule-type coverage (`cellIs`, `expression`, `colorScale`, `dataBar`, `iconSet`,
  `top10`, `unique`, `duplicate`, `containsText`, `timePeriod`, blanks/errors/
  nonBlanks), worksheet-global `priority` ordering, and read/write round-trip of
  `<conditionalFormatting>` + `<dxfs>`.

### Modified Capabilities

- `exceljs-parity`: v1.2.0 advances the parity matrix `conditional formatting`
  `targeted` → `shipped`. The release-recording requirement gains a v1.2.0 scenario.
<!-- `dxfs` is an internal styles surface, not a standalone capability; it is covered by the conditional-formatting capability and the styles read/write work, not as a separate matrix row. -->

## Impact

- **Code**: new model types in `src/model/conditional_formatting.rs` (mirrors
  `src/model/data_validation.rs`); new `Worksheet` methods in `src/model/worksheet.rs`
  near `add_data_validation` (L389); reader loop `parse_sheet_conditional_formattings`
  in `src/reader/xlsx.rs` (mirrors `parse_sheet_data_validations`) plus **`dxfs`
  parsing** in `src/reader/styles.rs` (replacing the current skip); writer emit in
  `src/writer/xlsx.rs` plus **`dxfs` emit** in `src/writer/styles.rs`
  (`emit_styles_xml`); napi bridge in `src/lib.rs` + `index.d.ts`.
- **APIs**: additive napi surface on `Worksheet`. No changes to existing public types.
- **Dependencies**: none added; reuses `quick-xml`, `napi`, and the existing
  `Font`/`Fill`/`Border` style primitives.
- **Specs**: new `conditional-formatting` capability spec; advances `exceljs-parity`
  matrix `conditional formatting` `targeted` → `shipped`.
- **Parity domain**: closes the last remaining "targeted" v1.x parity-matrix row
  (conditional formatting), leaving only `insert/splice rows` (v1.3.0) and the
  v2.0.0 capstone as unshipped matrix areas.
