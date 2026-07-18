# conditional-formatting Specification

## Purpose

Worksheet conditional formatting: `ws.addConditionalFormatting({ ref, rules })`
/ `ws.getConditionalFormatting()`, the `ConditionalFormat` / `CfRule` model, full
rule-type coverage (`cellIs`, `expression`, `colorScale`, `dataBar`, `iconSet`,
`top10`, `unique`, `duplicate`, `containsText`, `timePeriod`, blanks/errors/
nonBlanks), worksheet-global `priority` ordering, and read/write round-trip of
`<conditionalFormatting>` (worksheet XML) plus `<dxfs>` (styles XML).
Introduced by change `v1-2-0`.

## ADDED Requirements

### Requirement: Worksheet exposes conditional formatting add/get

A `Worksheet` SHALL expose `addConditionalFormatting(opts)` and
`getConditionalFormatting()`. `opts` SHALL accept `ref` (a cell range or
space-separated multi-range, e.g. `"A1:A10"` or `"A1:A10 C1:C10"`) and `rules`
(`CfRule[]`). `addConditionalFormatting` SHALL store the rules against `ref`;
`getConditionalFormatting()` SHALL return all stored formats as
`ConditionalFormat[]` (each `{ sqref, rules }`).

#### Scenario: Add a conditional format

- **WHEN** `ws.addConditionalFormatting({ ref: "A1:A4", rules: [{ type: "cellIs", operator: "lessThan", formula: [10], style: { font: { bold: true } } }] })`
- **THEN** `ws.getConditionalFormatting().length === 1`, the returned format's `sqref === "A1:A4"`, and its `rules[0].type === "cellIs"`

#### Scenario: Multiple ranges per call

- **WHEN** `ws.addConditionalFormatting({ ref: "A1:A4 C1:C4", rules: [{ type: "duplicate", style: { … } }] })`
- **THEN** `ws.getConditionalFormatting()[0].sqref === "A1:A4 C1:C4"`

### Requirement: cfRule supports all roadmap rule types

`CfRule` SHALL support, at minimum, these `type` values with their type-specific
fields: `cellIs` (`operator`, `formula[]`), `expression` (`formula[]`),
`colorScale` (`cfvo[]`, `color[]`), `dataBar` (`cfvo[]`, `color`), `iconSet`
(`iconSet`, `cfvo[]`), `top10` (`rank`, `percent?`, `bottom?`), `unique`,
`duplicate`, `containsText` (`operator`, `text`, `formula[]`), `timePeriod`
(`timePeriod`), `containsBlanks` / `notContainsBlanks`, `containsErrors` /
`notContainsErrors`. Every rule SHALL carry a worksheet-global unique `priority`.
Rules carrying a `style` (all except `colorScale` / `dataBar` / `iconSet`) SHALL
reference a differential format via `dxfId`.

#### Scenario: cellIs rule with style

- **WHEN** `addConditionalFormatting({ ref: "B1:B9", rules: [{ type: "cellIs", operator: "greaterThanOrEqual", formula: [0], style: { fill: { type: "pattern", pattern: "solid", bgColor: { argb: "FF00FF00" } } } }] })`
- **THEN** the stored rule has `type === "cellIs"`, `operator === "greaterThanOrEqual"`, `formula === ["0"]`, and a non-null `dxfId` pointing at a green fill `dxf`

#### Scenario: colorScale rule is inline (no dxfId)

- **WHEN** `addConditionalFormatting({ ref: "C1:C9", rules: [{ type: "colorScale", cfvo: [{ type: "min" }, { type: "max" }], color: [{ argb: "FFFF0000" }, { argb: "FF00FF00" }] }] })`
- **THEN** the stored rule has `type === "colorScale"`, `dxfId == null`, and two `cfvo` + two `color` entries

#### Scenario: iconSet and dataBar rules are inline

- **WHEN** rules of `type: "iconSet"` (`iconSet: "3TrafficLights"`, `cfvo: [...]`) and `type: "dataBar"` (`cfvo: [...], color: { argb: "…" }`) are added
- **THEN** both rules have `dxfId == null` and their `cfvo`/`color`/`iconSet` fields preserved

### Requirement: Writer emits worksheet conditionalFormatting plus dxfs

When a worksheet has conditional formats, the writer SHALL emit a
`<conditionalFormatting sqref="…">` element (containing one `<cfRule>` per rule
with `type`, `operator`, `priority`, `dxfId`, and child `<formula>`/`<cfvo>`/
`<colorScale>`/`<dataBar>`/`<iconSet>` as appropriate) at the schema-correct
position after `<sheetData>`. The writer SHALL emit a `<dxfs>` collection in
`xl/styles.xml` (after `cellXfs`, before `tableStyles`) for every differential
format referenced by `dxfId`, with `count` equal to the number of `dxfs`. A
workbook with no conditional formats SHALL NOT emit `<conditionalFormatting>`
elements or a `<dxfs>` part.

#### Scenario: Emit conditionalFormatting for a sheet with rules

- **WHEN** a worksheet has one conditional format with two rules
- **THEN** the sheet XML contains `<conditionalFormatting sqref="…">` with two `<cfRule>` children, each with a distinct `priority`, and `xl/styles.xml` contains a `<dxfs>` with at least the referenced `dxf` entries

#### Scenario: No rules omits parts

- **WHEN** no worksheet has conditional formats
- **THEN** no `<conditionalFormatting>` element and no `<dxfs>` element are emitted

### Requirement: Reader parses conditionalFormatting plus dxfs

The reader SHALL parse each sheet's `<conditionalFormatting sqref="…">` children
into `CfRule` objects (`type`, `operator`, `priority`, `dxfId`, `formula`,
`cfvo`, `color`, `iconSet`, `text`, `timePeriod`, `rank`, `percent`, `bottom`)
and resolve `dxfId` → the matching `dxf` in `xl/styles.xml` into a `style`. The
reader SHALL parse the `<dxfs>` collection (currently skipped) into the styles
model. A sheet without `<conditionalFormatting>` SHALL leave its conditional
format list empty.

#### Scenario: Read an Excel-authored conditional format

- **WHEN** a sheet XML carries `<conditionalFormatting sqref="A1:A10"><cfRule type="cellIs" operator="lessThan" priority="1" dxfId="0"><formula>10</formula></cfRule></conditionalFormatting>` and `styles.xml` has `<dxfs><dxf>…green fill…</dxf></dxfs>`
- **THEN** `ws.getConditionalFormatting()[0].sqref === "A1:A10"`, `rules[0].type === "cellIs"`, `rules[0].dxfId === 0`, and `rules[0].style` resolves to the green fill

#### Scenario: dxfs no longer skipped

- **WHEN** `styles.xml` contains a `<dxfs>` collection
- **THEN** the styles reader parses it into the `dxfs` model (instead of skipping it), so `dxfId` references resolve on round-trip

### Requirement: Priority ordering is preserved

`priority` SHALL be a worksheet-global, unique, 1-based integer. On read it SHALL
be taken verbatim from each `<cfRule priority="…">`. On write it SHALL be emitted
as stored (or, for excelrs-authored rules, assigned by document order). No two
rules in the same sheet SHALL share a `priority`.

#### Scenario: Read preserves priority

- **WHEN** a sheet has rules with `priority="3"` and `priority="1"`
- **THEN** `ws.getConditionalFormatting()` reports those rules with `priority` 3 and 1 respectively (order reflects read, not re-sorted)

#### Scenario: ExcelJS-authored rules get ordered priority

- **WHEN** `addConditionalFormatting` is called with an array of `N` rules
- **THEN** each emitted `<cfRule>` has a unique `priority` in `1..N` matching array order

### Requirement: Round-trip fidelity for conditional formatting

`excelrs` SHALL preserve, across a write then read, each rule's `type`, `operator`,
`priority`, `formula`/`cfvo`/`color`/`iconSet`/`text`/`timePeriod`/`rank`, and
resolved `style`, so the re-read model matches the source — whether the format
was authored by Excel or by ExcelJS.

#### Scenario: ExcelJS-authored format round-trips

- **WHEN** `ws.addConditionalFormatting({ ref, rules })` is written and re-read
- **THEN** the re-read format equals the source (ref, rule types, formulas/cfvo, styles)

#### Scenario: Excel-authored format round-trips

- **WHEN** an Excel-authored `.xlsx` with conditional formats is read, written, and re-read
- **THEN** the conditional formats (sqref, all rule types present, priorities, dxf styles) are preserved

#### Scenario: Non-cf dxfs are not dropped

- **WHEN** a source workbook contains `dxfs` not referenced by any `cfRule` (e.g. pivot-table dxfs)
- **THEN** those `dxfs` are preserved on write (count and content unchanged) so the file stays valid
