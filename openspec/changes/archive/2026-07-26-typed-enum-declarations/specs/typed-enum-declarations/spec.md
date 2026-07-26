## ADDED Requirements

### Requirement: Fill kind accepts only valid values

The `Fill.kind` property SHALL accept only `"none"`, `"solid"`, `"pattern"`, or `"gradient"` at the TypeScript type level. Rust-side validation SHALL remain unchanged (already rejects invalid values).

#### Scenario: Valid fill kind in TypeScript

- **WHEN** TypeScript code assigns `"solid"` to `Fill.kind`
- **THEN** the assignment compiles without type error

#### Scenario: Invalid fill kind rejected by TypeScript

- **WHEN** TypeScript code assigns `"invalid"` to `Fill.kind`
- **THEN** the assignment SHALL produce a compile-time type error

### Requirement: Border style accepts only valid line styles

The `BorderStyle.style` property SHALL accept only `"thin"`, `"medium"`, `"thick"`, `"dashed"`, `"dotted"`, or `"double"` at the TypeScript type level. `"none"` SHALL remain rejected (as today; use `null`/`undefined` side instead).

#### Scenario: Valid border style in TypeScript

- **WHEN** TypeScript code assigns `"thin"` to `BorderStyle.style`
- **THEN** the assignment compiles without type error

#### Scenario: Invalid border style rejected

- **WHEN** TypeScript code assigns `"none"` to `BorderStyle.style`
- **THEN** the assignment SHALL produce a compile-time type error

### Requirement: Alignment horizontal accepts only valid values

The `Alignment.horizontal` property SHALL accept only `"left"`, `"center"`, `"right"`, `"fill"`, or `"justify"` (or `null`/`undefined`) at the TypeScript type level.

#### Scenario: Valid horizontal alignment

- **WHEN** TypeScript code assigns `"center"` to `Alignment.horizontal`
- **THEN** the assignment compiles without type error

### Requirement: Alignment vertical accepts only valid values

The `Alignment.vertical` property SHALL accept only `"top"`, `"middle"`, or `"bottom"` (or `null`/`undefined`) at the TypeScript type level.

#### Scenario: Valid vertical alignment

- **WHEN** TypeScript code assigns `"top"` to `Alignment.vertical`
- **THEN** the assignment compiles without type error

### Requirement: Fill gradient type accepts only valid values

The `Fill.gradientType` property SHALL accept only `"linear"` or `"path"` (or `null`/`undefined`) at the TypeScript type level.

#### Scenario: Valid gradient type

- **WHEN** TypeScript code assigns `"linear"` to `Fill.gradientType`
- **THEN** the assignment compiles without type error

### Requirement: Image anchor type accepts only valid values

The `ImageAnchor.anchorType` property SHALL accept only `"oneCell"` or `"twoCell"` at the TypeScript type level.

#### Scenario: Valid anchor type

- **WHEN** TypeScript code assigns `"oneCell"` to `ImageAnchor.anchorType`
- **THEN** the assignment compiles without type error

### Requirement: Sheet view state accepts only valid values

The `SheetView.state` property SHALL accept only `"frozen"`, `"split"`, or `""` (or `null`/`undefined`) at the TypeScript type level.

#### Scenario: Valid sheet view state

- **WHEN** TypeScript code assigns `"frozen"` to `SheetView.state`
- **THEN** the assignment compiles without type error

### Requirement: Sheet view active pane accepts only valid values

The `SheetView.activePane` property SHALL accept only `"bottomLeft"`, `"bottomRight"`, `"topLeft"`, or `"topRight"` (or `null`/`undefined`) at the TypeScript type level.

#### Scenario: Valid active pane

- **WHEN** TypeScript code assigns `"topLeft"` to `SheetView.activePane`
- **THEN** the assignment compiles without type error

### Requirement: Page setup orientation accepts only valid values

The `PageSetup.orientation` property SHALL accept only `"portrait"` or `"landscape"` (or `null`/`undefined`) at the TypeScript type level.

#### Scenario: Valid orientation

- **WHEN** TypeScript code assigns `"landscape"` to `PageSetup.orientation`
- **THEN** the assignment compiles without type error

### Requirement: Cell value type accepts only valid discriminants

The `CellValue.valueType` property SHALL accept only `"Null"`, `"Number"`, `"String"`, `"Boolean"`, `"Formula"`, `"Error"`, `"Hyperlink"`, `"RichText"`, `"Merge"`, or `"Date"` at the TypeScript type level.

#### Scenario: Valid value type

- **WHEN** TypeScript code assigns `"Number"` to `CellValue.valueType`
- **THEN** the assignment compiles without type error

#### Scenario: Invalid value type rejected

- **WHEN** TypeScript code assigns `"Invalid"` to `CellValue.valueType`
- **THEN** the assignment SHALL produce a compile-time type error

### Requirement: CfRule type accepts only valid rule types

The `CfRule.type` property SHALL accept only `"cellIs"`, `"expression"`, `"colorScale"`, `"dataBar"`, `"iconSet"`, `"top10"`, `"unique"`, `"duplicate"`, `"containsText"`, `"timePeriod"`, `"containsBlanks"`, `"notContainsBlanks"`, `"containsErrors"`, or `"notContainsErrors"` at the TypeScript type level.

#### Scenario: Valid cf rule type

- **WHEN** TypeScript code assigns `"cellIs"` to `CfRule.type`
- **THEN** the assignment compiles without type error

### Requirement: Cfvo type accepts only valid value types

The `Cfvo.type` property SHALL accept only `"num"`, `"percent"`, `"percentile"`, `"formula"`, `"min"`, `"max"`, `"autoMin"`, or `"autoMax"` at the TypeScript type level.

#### Scenario: Valid cfvo type

- **WHEN** TypeScript code assigns `"percent"` to `Cfvo.type`
- **THEN** the assignment compiles without type error

### Requirement: Data validation type accepts only valid types

The `DataValidation.type` property SHALL accept only `"whole"`, `"decimal"`, `"list"`, `"date"`, `"time"`, `"textLength"`, or `"custom"` at the TypeScript type level.

#### Scenario: Valid data validation type

- **WHEN** TypeScript code assigns `"list"` to `DataValidation.type`
- **THEN** the assignment compiles without type error

### Requirement: Data validation error style accepts only valid styles

The `DataValidation.errorStyle` property SHALL accept only `"information"`, `"warning"`, or `"stop"` (or `null`/`undefined`) at the TypeScript type level.

#### Scenario: Valid error style

- **WHEN** TypeScript code assigns `"stop"` to `DataValidation.errorStyle`
- **THEN** the assignment compiles without type error

### Requirement: Generated types survive regeneration

The Rust `#[napi(string_enum)]` conversions SHALL produce string literal union types in the auto-generated `index.d.ts` that match the exact string values used at runtime.

#### Scenario: Regeneration preserves types

- **WHEN** `pnpm build` (or equivalent napi build command) regenerates `index.d.ts`
- **THEN** the string literal union types for converted enums SHALL be present in the generated output
- **THEN** the generated string values SHALL match the runtime Rust enum variant names
