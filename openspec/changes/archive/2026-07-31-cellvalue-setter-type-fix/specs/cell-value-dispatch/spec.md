## ADDED Requirements

### Requirement: Setter compile-time type rejects cross-variant field combinations

The `Cell.value` setter's TypeScript signature SHALL reject object-shaped values that mix
fields from different `CellValue` union variants at compile time. A `CellValueInput`
input type SHALL replace `Partial<CellValue>` so that `Partial` no longer
distributes over the discriminated union, which would otherwise allow
`{ valueType: "Number", string: "leaked" }`.

#### Scenario: Cross-variant fields rejected at compile time

- **WHEN** a consumer writes `cell.value = { valueType: "Number", string: "leaked" }`
- **THEN** the TypeScript compiler SHALL reject the excess `string` property

#### Scenario: Omitting valueType still allowed for backward compat

- **WHEN** a consumer writes `cell.value = { number: 42 }` (no `valueType`)
- **THEN** the TypeScript compiler SHALL accept it and the setter SHALL infer `valueType: "Number"` from shape

#### Scenario: Variant field required when discriminant present

- **WHEN** a consumer writes `cell.value = { valueType: "Number" }` (discriminant without its variant field)
- **THEN** the TypeScript compiler SHALL reject the missing `number` property
