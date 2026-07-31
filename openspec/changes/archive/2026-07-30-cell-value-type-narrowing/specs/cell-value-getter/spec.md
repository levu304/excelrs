## MODIFIED Requirements

### Requirement: TypeScript getter type is a discriminated union

The generated `Cell.value` getter TypeScript type SHALL be `CellValueResult` —
`number | string | boolean | Date | null | CellValue` — where `CellValue` is itself a
discriminated union narrowable on `valueType`, so rich-type shapes stay fully typed
without a cast. Narrowing is performed on the `CellValue.valueType` field (via
`cell.valueOf` — a getter, not a method), not on the separate `Cell.type` accessor —
TypeScript cannot narrow a class getter's return type from a sibling instance property.

#### Scenario: Narrowing works on valueType

- **WHEN** a consumer writes `const cv = cell.valueOf; if (cv.valueType === "RichText") { cv.richText; }`
- **THEN** the TypeScript compiler SHALL accept `cv.richText` as `RichTextRun[]` inside the branch without a cast

#### Scenario: Invalid variant combos are rejected at compile time

- **WHEN** a consumer writes `cell.value = { valueType: "RichText", number: 42 }`
- **THEN** the TypeScript compiler SHALL reject the excess `number` property (the `CellValue` union has no branch with both `valueType: "RichText"` and `number`)

## ADDED Requirements

### Requirement: cell.valueOf returns the full CellValue

The `Cell.valueOf` getter SHALL return the full `CellValue` discriminated union for the
cell, regardless of variant. This is the typed escape hatch for rich cells when the caller
wants the whole object rather than the unwrapped primitive that `cell.value` returns.

#### Scenario: Read a RichText cell via valueOf

- **WHEN** a RichText cell stores two runs
- **THEN** `cell.valueOf` SHALL be a `CellValue` whose `valueType` is `"RichText"` and whose `richText` equals the two runs, and `if (cv.valueType === "RichText") cv.richText` SHALL type-check without a cast

### Requirement: cell.richText returns runs directly for RichText cells

The `Cell.richText` getter SHALL return `RichTextRun[] | null` — the parsed runs when the
cell is a RichText cell, or `null` otherwise. No cast SHALL be required to read the runs.

#### Scenario: Read runs without a cast

- **WHEN** a RichText cell stores two runs and `cell.type === "RichText"`
- **THEN** `cell.richText` SHALL be the `RichTextRun[]` (length 2) with the bold flag on run 0, typed without any cast

#### Scenario: Non-RichText cell returns null

- **WHEN** a Number cell stores `42`
- **THEN** `cell.richText` SHALL be `null`
