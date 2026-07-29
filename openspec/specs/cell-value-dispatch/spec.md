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

The public `Cell.value` setter SHALL accept a formula object.

#### Scenario: Assign formula via object

- **WHEN** `cell.value = { formula: "SUM(A1:B1)" }` is assigned
- **THEN** `cell.value.value_type` SHALL be `"Formula"` with the formula string preserved

### Requirement: Public setter preserves explicit discriminant for round-trip

The public `Cell.value` setter SHALL honor an explicit `valueType` so a read-back `CellValue` can be reassigned.

#### Scenario: Round-trip a read-back object

- **WHEN** `cell.value = { valueType: "Date", dateSerial: 45458.5 }` (or any `CellValue` object returned by the getter) is assigned
- **THEN** the stored `value_type` and fields SHALL equal the assigned object (no loss to Null)

### Requirement: Public setter still accepts primitives

The public `Cell.value` setter SHALL preserve existing primitive behavior.

#### Scenario: Assign primitives and Date

- **WHEN** `cell.value = 42`, `= "s"`, `= true`, `= new Date(...)`, or `= null` is assigned
- **THEN** `value_type` SHALL be Number, String, Boolean, Date, or Null respectively (unchanged from current behavior)
