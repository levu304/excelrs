# cell-value-dispatch Specification

## Purpose

The public `Cell.value` setter dispatches object-shaped cell values (RichText, Hyperlink, Formula) and honors explicit `valueType` discriminant round-trip, via shape inference. Primitives (Number, String, Boolean, Date, Null) continue to work as before.

## Requirements

### Requirement: Public setter routes object-shaped rich text

The public `Cell.value` setter SHALL accept a rich-text object and store it as a RichText cell value.

#### Scenario: Assign rich text via object

- **WHEN** `cell.value = { richText: [{ text: "Hello ", font: { bold: true } }, { text: "World" }] }` is assigned
- **THEN** the stored `cell.value.value_type` SHALL be `"RichText"` and `cell.value.rich_text` SHALL equal the two given runs with the bold flag preserved on the first run

### Requirement: Public setter routes object-shaped hyperlink

The public `Cell.value` setter SHALL accept a hyperlink object.

#### Scenario: Assign hyperlink via object

- **WHEN** `cell.value = { hyperlink: "https://example.com", hyperlinkText: "Example" }` is assigned
- **THEN** `cell.value.value_type` SHALL be `"Hyperlink"` with the URL and display text preserved

### Requirement: Public setter routes object-shaped formula

The public `Cell.value` setter SHALL accept a formula object and SHALL persist it to
the XLSX `<f>` element so the formula survives a write and read-back, not just in memory.

#### Scenario: Assign formula via object

- **WHEN** `cell.value = { formula: "SUM(A1:B1)" }` is assigned
- **THEN** `cell.value.value_type` SHALL be `"Formula"` with the formula string preserved

#### Scenario: Formula persists through XLSX write and read-back

- **WHEN** `cell.value = { formula: "SUM(A1:B1)" }` is assigned, the workbook is written and read back
- **THEN** the re-read cell SHALL be a formula cell whose `formula` equals `"SUM(A1:B1)"` and the emitted XML SHALL contain a `<f>SUM(A1:B1)</f>` element

### Requirement: Public setter preserves explicit discriminant for round-trip

The public `Cell.value` setter SHALL honor an explicit `valueType` so a read-back `CellValue` can be reassigned.

#### Scenario: Round-trip a read-back object

- **WHEN** `cell.value = { valueType: "Date", dateSerial: 45458.5 }` (or any `CellValue` object returned by the getter) is assigned
- **THEN** the stored `value_type` and fields SHALL equal the assigned object (no loss to Null)

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

### Requirement: Public setter still accepts primitives

The public `Cell.value` setter SHALL preserve existing primitive behavior.

#### Scenario: Assign primitives and Date

- **WHEN** `cell.value = 42`, `= "s"`, `= true`, `= new Date(...)`, or `= null` is assigned
- **THEN** `value_type` SHALL be Number, String, Boolean, Date, or Null respectively (unchanged from current behavior)
