# conditional-formatting Specification

## Purpose
Defines conditional formatting: the `ws.addConditionalFormatting` / `getConditionalFormatting` API, the rule types excelrs supports, and the `<conditionalFormatting>` + `dxfs` OOXML pair they serialize to, with priority ordering and round-trip fidelity preserved.
## Requirements
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

### Requirement: CfRule supports every roadmap rule type

`CfRule` SHALL support, at minimum, the roadmap's `cellIs`, `expression`, `colorScale`,
`dataBar`, `iconSet`, `top10`, `unique`, `duplicate`, `containsText`, `timePeriod`,
`containsBlanks`, `notContainsBlanks`, `containsErrors`, and `notContainsErrors` types.

#### Scenario: Every roadmap rule type is representable

- **WHEN** a caller constructs a `CfRule` for any listed type
- **THEN** the constructed rule SHALL be representable in the public `CfRule` type

### Requirement: Conditional formatting rule types and their fields

Each `CfRule` type SHALL carry the type-specific fields for that rule: `cellIs`
(`operator`, `formula[]`), `expression` (`formula[]`), `colorScale` (`cfvo[]`, `color[]`),
`dataBar` (`cfvo[]`, `color`), `iconSet` (`iconSet`, `cfvo[]`), `top10` (`rank`,
`percent?`, `bottom?`), `unique`, `duplicate`, `containsText` (`operator`, `text`,
`formula[]`), `timePeriod` (`timePeriod`), `containsBlanks` / `notContainsBlanks`, and
`containsErrors` / `notContainsErrors`.

#### Scenario: cellIs rule carries its operator and formulas

- **WHEN** a `cellIs` rule is constructed
- **THEN** it SHALL carry an `operator` and a `formula[]` field

#### Scenario: dataBar rule carries cfvo and color

- **WHEN** a `dataBar` rule is constructed
- **THEN** it SHALL carry a `cfvo[]` and a `color` field

#### Scenario: top10 rule carries rank with optional flags

- **WHEN** a `top10` rule is constructed
- **THEN** it SHALL carry a `rank` field and optional `percent` and `bottom` flags

### Requirement: Rules carrying a style reference a differential format

Every `CfRule` SHALL carry a worksheet-global unique `priority`. A rule carrying a `style`
— every type except `colorScale`, `dataBar`, and `iconSet` — SHALL reference a differential
format via `dxfId`.

#### Scenario: Two rules on one worksheet get distinct priorities

- **WHEN** two `CfRule`s are added to the same worksheet
- **THEN** their `priority` values SHALL be distinct within that worksheet

#### Scenario: Style-bearing rule references a dxfId

- **WHEN** a `cellIs` rule is constructed with a style
- **THEN** it SHALL carry a `dxfId` referencing a differential format

#### Scenario: Scaleless rule types omit dxfId

- **WHEN** a `colorScale`, `dataBar`, or `iconSet` rule is constructed
- **THEN** it SHALL NOT reference a differential format via `dxfId`

### Requirement: Writer emits worksheet conditionalFormatting

When a worksheet has conditional formats, the writer SHALL emit a
`<conditionalFormatting sqref="…">` element at the schema-correct position after
`<sheetData>`, containing one `<cfRule>` per rule with its `type`, `operator`, `priority`,
`dxfId`, and the child `<formula>` / `<cfvo>` / `<colorScale>` / `<dataBar>` / `<iconSet>
elements appropriate to that rule.

#### Scenario: Worksheet with one rule emits one conditionalFormatting element

- **WHEN** a worksheet has one conditional-format rule
- **THEN** the writer SHALL emit exactly one `<conditionalFormatting sqref="…">` element after `<sheetData>` containing that rule's `<cfRule>`

#### Scenario: colorScale rule emits its scale children

- **WHEN** a worksheet has a `colorScale` rule
- **THEN** the emitted `<cfRule>` SHALL contain `<colorScale>` with its `<cfvo>` and color children

### Requirement: Writer emits dxfs for every referenced differential format

The writer SHALL emit a `<dxfs>` collection in `xl/styles.xml` positioned after `cellXfs`
and before `tableStyles`, containing one differential format per `dxfId` referenced by the
workbook's rules, with `count` equal to the number of `dxfs`.

#### Scenario: Referenced dxfIds produce a dxfs collection

- **WHEN** the workbook's rules reference three differential formats
- **THEN** the writer SHALL emit a `<dxfs>` collection with `count="3"` after `cellXfs` and before `tableStyles`

### Requirement: Workbook without conditional formats emits no conditional-formatting parts

A workbook whose worksheets carry no conditional formats SHALL NOT emit
`<conditionalFormatting>` elements or a `<dxfs>` part.

#### Scenario: Plain workbook has no dxfs part

- **WHEN** a workbook with no conditional formats is written
- **THEN** `xl/styles.xml` SHALL contain no `<dxfs>` element

#### Scenario: Plain worksheet has no conditionalFormatting element

- **WHEN** a worksheet with no conditional formats is written
- **THEN** the sheet XML SHALL contain no `<conditionalFormatting>` element

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
