# cell-value-getter Specification

## Purpose

Defines the public `Cell.value` getter contract — what `cell.value` returns for each
cell-value variant — and the companion `Cell.type` discriminant accessor. Replaces the
historical behavior where the getter always returned a flat `CellValue` wrapper object.

## Requirements

### Requirement: Getter returns the primitive for Number cells

The `Cell.value` getter SHALL return a JavaScript `number` for Number-type cells.

#### Scenario: Read a Number cell

- **WHEN** a Number cell has stored value `42`
- **THEN** `cell.value` SHALL be the primitive `42` (a `number`, not a `CellValue` object)

#### Scenario: Numeric operations work directly

- **WHEN** `cell.value` is a Number cell returning `42`
- **THEN** `cell.value + 1` SHALL evaluate to `43` and `typeof cell.value` SHALL be `"number"`

### Requirement: Getter returns the primitive for String cells

The `Cell.value` getter SHALL return a JavaScript `string` for String-type cells.

#### Scenario: Read a String cell

- **WHEN** a String cell has stored value `"hello"`
- **THEN** `cell.value` SHALL be the primitive `"hello"` (a `string`)

### Requirement: Getter returns the primitive for Boolean cells

The `Cell.value` getter SHALL return a JavaScript `boolean` for Boolean-type cells.

#### Scenario: Read a Boolean cell

- **WHEN** a Boolean cell has stored value `true`
- **THEN** `cell.value` SHALL be the primitive `true` (a `boolean`)

### Requirement: Getter returns a JS Date for Date cells

The `Cell.value` getter SHALL return a JavaScript `Date` instance for Date-type cells
(converted from the stored Excel serial via `JsDate.to_unknown()`).

#### Scenario: Read a Date cell

- **WHEN** a Date cell stores serial `45458.5` (2024-06-15T12:00:00Z)
- **THEN** `cell.value` SHALL be a `Date` instance equal to that instant and `cell.value instanceof Date` SHALL be `true`

### Requirement: Getter returns null for Null cells

The `Cell.value` getter SHALL return JavaScript `null` for Null-type cells.

#### Scenario: Read an empty cell

- **WHEN** a cell has no value (`value_type === "Null"`)
- **THEN** `cell.value` SHALL be `null`

### Requirement: Getter returns a CellValue object for rich/compound cells

The `Cell.value` getter SHALL return a `CellValue` object (with `valueType` discriminant
and variant fields) for Formula, RichText, Hyperlink, Error, and Merge cells.

#### Scenario: Read a Formula cell

- **WHEN** a Formula cell stores `formula = "SUM(A1:A2)"`
- **THEN** `cell.value` SHALL be a `CellValue` object whose `valueType` is `"Formula"` and whose `formula` is `"SUM(A1:A2)"`

#### Scenario: Read a RichText cell

- **WHEN** a RichText cell stores two runs
- **THEN** `cell.value` SHALL be a `CellValue` object whose `valueType` is `"RichText"` and whose `richText` equals the two runs

### Requirement: TypeScript getter type is a discriminated union

The generated `Cell.value` getter TypeScript type SHALL be `CellValueResult` —
`number | string | boolean | Date | null | CellValue` — so reads narrow correctly and
rich-type shapes stay fully typed.

#### Scenario: Narrowing works in TypeScript

- **WHEN** a consumer writes `if (cell.type === "Number") { const n: number = cell.value; }`
- **THEN** the TypeScript compiler SHALL accept `cell.value` as `number` inside the branch without a cast

### Requirement: New `Cell.type` accessor returns the discriminant

The `Cell.type` getter SHALL return a `CellType` string-enum value
(`"Null" | "Number" | "String" | "Boolean" | "Date" | "Formula" | "Error" | "Hyperlink" | "RichText" | "Merge"`)
identifying the cell's value variant. This replaces `cell.value.valueType` for primitives.

#### Scenario: Discriminant for a Number cell

- **WHEN** a Number cell stores `42`
- **THEN** `cell.type` SHALL be `"Number"` and `cell.value` SHALL be the primitive `42`

#### Scenario: Discriminant for a Formula cell

- **WHEN** a Formula cell stores `formula = "SUM(A1:A2)"`
- **THEN** `cell.type` SHALL be `"Formula"` and `cell.value.valueType` SHALL also be `"Formula"`

### Requirement: `Cell.date` getter remains available

The `Cell.date` getter SHALL continue to return `Date | null` for Date-type cells, kept for
backward compatibility. It is redundant now that `Cell.value` returns a `Date` for Date
cells and is marked `@deprecated` for removal in v3.

#### Scenario: Date cell reachable via both accessors

- **WHEN** a Date cell stores serial `45458.5`
- **THEN** both `cell.value` (as `Date`) and `cell.date` (as `Date`) SHALL equal the same instant
