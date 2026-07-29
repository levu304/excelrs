# cell-value-dispatch Specification (delta)

## MODIFIED Requirements

### Requirement: Public setter routes object-shaped formula

The public `Cell.value` setter SHALL accept a formula object and SHALL persist it to
the XLSX `<f>` element so the formula survives a write and read-back, not just in memory.

#### Scenario: Assign formula via object

- **WHEN** `cell.value = { formula: "SUM(A1:B1)" }` is assigned
- **THEN** `cell.value.value_type` SHALL be `"Formula"` with the formula string preserved

#### Scenario: Formula persists through XLSX write and read-back

- **WHEN** `cell.value = { formula: "SUM(A1:B1)" }` is assigned, the workbook is written and read back
- **THEN** the re-read cell SHALL be a formula cell whose `formula` equals `"SUM(A1:B1)"` and the emitted XML SHALL contain a `<f>SUM(A1:B1)</f>` element

## ADDED Requirements

### Requirement: Public setter validates explicit valueType discriminant

The public `Cell.value` setter SHALL reject an object whose `valueType` is not a known
discriminant (`Number`, `String`, `Boolean`, `Formula`, `Error`, `Hyperlink`,
`RichText`, `Date`, `Null`, `Merge`) by raising an error, rather than silently
producing an empty cell.

#### Scenario: Unknown valueType raises an error

- **WHEN** `cell.value = { valueType: "Banana", number: 5 }` is assigned
- **THEN** the setter SHALL raise an error and MUST NOT leave the cell value as an empty `<c>`

### Requirement: Public setter clears stale formula on reassignment

The public `Cell.value` setter SHALL clear any previously-set formula when a
non-formula value is assigned, so a formula cell is not emitted under a new value.

#### Scenario: Replacing a formula with a primitive clears the formula

- **WHEN** `cell.value = { formula: "SUM(A1:A2)" }` is assigned and then `cell.value = 42` is assigned
- **THEN** the cell SHALL be a Number cell and MUST NOT emit a `<f>` element on write
