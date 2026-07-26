## ADDED Requirements

### Requirement: Reader preserves OOXML pattern type name

The reader SHALL store the raw OOXML `patternType` attribute value into `Fill.pattern` in addition to mapping it to `FillKind`.

#### Scenario: Read gray125 pattern fill

- **WHEN** parsing `<patternFill patternType="gray125"/>`
- **THEN** `fill.kind` SHALL be `FillKind::None` (via catch-all)
- **THEN** `fill.pattern` SHALL be `Some("gray125")`

#### Scenario: Read lightHorizontal pattern fill

- **WHEN** parsing `<patternFill patternType="lightHorizontal"/>`
- **THEN** `fill.pattern` SHALL be `Some("lightHorizontal")`

#### Scenario: Read solid pattern fill

- **WHEN** parsing `<patternFill patternType="solid"/>`
- **THEN** `fill.kind` SHALL be `FillKind::Solid`
- **THEN** `fill.pattern` SHALL be `Some("solid")`

### Requirement: Writer emits pattern type name from Fill.pattern

The writer SHALL use `Fill.pattern` (not `FillKind` Display) as the `patternType` attribute value. When `Fill.pattern` is `None`, the writer SHALL fall back to `"solid"`.

#### Scenario: Write with explicit pattern name

- **WHEN** writing a fill with `fill.pattern = Some("gray125")` and `fill.kind = FillKind::None`
- **THEN** the emitted XML SHALL contain `patternType="gray125"`

#### Scenario: Write with no pattern set

- **WHEN** writing a fill with `fill.pattern = None`, `fill.foreground = Some("FFFFFF00")`
- **THEN** the emitted XML SHALL contain `patternType="solid"` (OOXML default)

#### Scenario: DXF write with pattern name

- **WHEN** writing a DXF fill with `fill.pattern = Some("gray125")`
- **THEN** the emitted DXF XML SHALL contain `patternType="gray125"`
