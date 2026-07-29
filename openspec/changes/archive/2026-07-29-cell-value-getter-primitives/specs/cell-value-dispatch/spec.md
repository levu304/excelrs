# cell-value-dispatch Specification (delta)

## MODIFIED Requirements

### Requirement: Public setter routes object-shaped rich text

public `Cell.value` setter SHALL accept rich-text object store RichText cell value.

#### Scenario: Assign rich text via object

- **WHEN** `cell.value = { richText: [{ text: "Hello ", font: { bold: true } }, { text: "World" }] }` assigned
- **THEN** stored `cell.value.value_type` SHALL be `"RichText"` and `cell.value.rich_text` SHALL equal two runs with bold flag preserved on first run

> **Modified**: After assignment, `cell.type` SHALL be `"RichText"`. `cell.value` remains a `CellValue` object (rich types are unchanged); the `value_type` field is still readable on the returned object.

### Requirement: Public setter routes object-shaped hyperlink

public `Cell.value` setter SHALL accept hyperlink object.

#### Scenario: Assign hyperlink object

- **WHEN** `cell.value = { hyperlink: "https://example.com", hyperlinkText: "Example" }` assigned
- **THEN** `cell.value.value_type` SHALL be `"Hyperlink"` with URL and display text preserved

> **Modified**: After assignment, `cell.type` SHALL be `"Hyperlink"`. `cell.value` remains a `CellValue` object.

### Requirement: Public setter routes object-shaped formula

public `Cell.value` setter SHALL accept formula object SHALL persist XLSX `<f>` element formula survives write read-back, not just in memory.

#### Scenario: Assign formula object

- **WHEN** `cell.value = { formula: "SUM(A1:B1)" }` assigned
- **THEN** `cell.value.value_type` SHALL be `"Formula"` with formula string preserved

#### Scenario: Formula persists through XLSX write read-back

- **WHEN** `cell.value = { formula: "SUM(A1:B1)" }` assigned, workbook written and read back
- **THEN** re-read SHALL `"SUM(A1:B1)"`, XML SHALL `<f>SUM(A1:B1)</f>` round-trip, and `cell.value` SHALL read-back as a `CellValue`

> **Modified**: After assignment, `cell.type` SHALL be `"Formula"`. `cell.value` remains a `CellValue` object.

### Requirement: Public setter honors explicit valueType discriminant

public `Cell.value` setter SHALL honor explicit `valueType` so a read-back `CellValue` can be reassigned, and SHALL reject object whose `valueType` is not a known discriminant (`Number`, `String`, `Boolean`, `Formula`, `Error`, `Hyperlink`, `RichText`, `Date`, `Null`, `Merge`), raising an error rather than silently producing an empty cell.

#### Scenario: Unknown valueType raises error

- **WHEN** `cell.value = { valueType: "Banana", number: 5 }` assigned
- **THEN** setter SHALL raise error and MUST NOT leave cell value empty

### Requirement: Public setter clears stale formula on reassignment

public `Cell.value` setter SHALL clear any previously-set formula when a non-formula value is assigned, so a formula cell is not emitted under the new value.

#### Scenario: Replacing formula with primitive clears formula

- **WHEN** `cell.value = { formula: "SUM(A1:A2)" }` assigned then `cell.value = 42` assigned
- **THEN** cell SHALL be a Number cell, MUST NOT emit `<f>` on write, and `cell.type` SHALL be `"Number"` (was previously checked via `cell.value.value_type`)

> **Modified**: The discriminant is now read via `cell.type` (`"Number"`) rather than `cell.value.value_type`, because `cell.value` returns the primitive `42` for Number cells.

### Requirement: Public setter still accepts primitives

public `Cell.value` setter SHALL preserve existing primitive behavior: `42` → Number, `"s"` → String, `true` → Boolean, `new Date(...)` → Date, `null` → Null.

#### Scenario: Assign primitives

- **WHEN** `cell.value = 42`, `= "s"`, `= true`, `= new Date(...)`, `= null` assigned
- **THEN** `value_type` SHALL be Number, String, Boolean, Date, Null respectively

> **Modified**: After assignment, `cell.value` (getter) returns the **primitive** for these variants — `42`, `"s"`, `true`, a `Date` instance, `null` — not a `CellValue` wrapper. Use `cell.type` to discriminate. This is a breaking change to the getter contract paired with this setter; see the `cell-value-getter` capability.

## ADDED Requirements

### Requirement: Setter and getter contracts are paired via `cell.type`

The `Cell.value` getter and setter SHALL agree via the `cell.type` discriminant: whatever variant the setter stored, `cell.type` SHALL report that variant and the getter SHALL return the corresponding shape (primitive for Number/String/Boolean/Date/Null, `CellValue` object for Formula/RichText/Hyperlink/Error/Merge).

#### Scenario: Round-trip discriminant consistency

- **WHEN** `cell.value = 42` is assigned
- **THEN** `cell.type` SHALL be `"Number"` and `cell.value` SHALL be the primitive `42`; re-reading after an XLSX write/read SHALL yield the same `cell.type` and primitive shape.
