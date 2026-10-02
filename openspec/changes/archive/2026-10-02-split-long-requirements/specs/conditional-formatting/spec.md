## REMOVED Requirements

### Requirement: cfRule supports all roadmap rule types

**Reason**: A single requirement enumerated twelve `CfRule` type values with their type-specific fields plus cross-cutting rules, producing over-long prose.

**Migration**: Replaced by "CfRule supports every roadmap rule type", "Rules carrying a style reference a differential format", and "Conditional formatting rule types and their fields".

### Requirement: Writer emits worksheet conditionalFormatting plus dxfs

**Reason**: Bundled the `<conditionalFormatting>` emission, the `<dxfs>` emission, and the no-formats negative case into one over-long requirement.

**Migration**: Replaced by "Writer emits worksheet conditionalFormatting", "Writer emits dxfs for every referenced differential format", and "Workbook without conditional formats emits no conditional-formatting parts".

## ADDED Requirements

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