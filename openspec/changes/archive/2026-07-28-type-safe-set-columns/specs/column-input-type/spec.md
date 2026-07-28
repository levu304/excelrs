## ADDED Requirements

### Requirement: ColumnInput interface

The system SHALL expose a `ColumnInput` interface (TS) / `#[napi(object)]` struct (Rust) that accepts plain JS objects with optional fields matching the ExcelJS column descriptor shape.

The `ColumnInput` SHALL support the following fields, all optional:

- `colNum` (number): 1-indexed column position. `0` or omitted means auto-assigned.
- `header` (string): Column header label.
- `key` (string): Data-binding key for row values.
- `width` (number): Column width in characters.
- `hidden` (boolean): Column visibility.
- `style` (Style): Column-level default style.
- `outlineLevel` (number): Outline/grouping level, clamped 0–7.

#### Scenario: Plain object accepted

- **WHEN** caller passes `{ header: "Name", key: "name", width: 20 }` to `setColumns`
- **THEN** the system SHALL accept it without requiring `new Column(...)` constructor

#### Scenario: All fields optional

- **WHEN** caller passes `{}` (empty object) as a column descriptor
- **THEN** the system SHALL accept it and apply defaults (colNum auto-assigned, empty header/key, width 0, hidden false, no style)

#### Scenario: camelCase property mapping

- **WHEN** caller passes `{ colNum: 5, outlineLevel: 2 }`
- **THEN** the system SHALL map camelCase JS property names to the corresponding Rust fields

### Requirement: Type-safe setColumns signature

The system SHALL change `Worksheet.setColumns` signature from `cols: any` to `cols: ColumnInput[]`.

#### Scenario: TypeScript compilation of typed call

- **WHEN** caller writes `ws.setColumns([{ header: "A", key: "a", width: 10 }])`
- **THEN** TypeScript compiler SHALL accept the call without error

#### Scenario: TypeScript rejection of invalid shape

- **WHEN** caller writes `ws.setColumns([{ unknownField: true }])`
- **THEN** TypeScript compiler SHALL emit a type error

#### Scenario: TypeScript rejection of primitive

- **WHEN** caller writes `ws.setColumns("not an array")`
- **THEN** TypeScript compiler SHALL emit a type error

### Requirement: ColumnInput validation

The system SHALL validate `ColumnInput` array elements identically to the current serde-based validation, including:

- Auto-assign `colNum` for entries where `colNum` is 0 or omitted, starting from `max(existing col_nums) + 1`
- Reject duplicate `colNum` values within the same call
- Validate `style` field using the same rules as `Cell.setStyle`

#### Scenario: Auto-assign sequential colNum

- **WHEN** caller passes `[{ header: "A" }, { header: "B" }]` with both colNum omitted
- **THEN** first column SHALL get colNum = 1, second column SHALL get colNum = 2 (or next available from existing columns)

#### Scenario: Reject duplicate colNum

- **WHEN** caller passes `[{ colNum: 3 }, { colNum: 3 }]`
- **THEN** system SHALL throw an error with message including "duplicate col_num 3"

#### Scenario: Invalid style rejected

- **WHEN** caller passes `[{ header: "X", style: { numFmt: "" } }]` (empty numFmt is invalid)
- **THEN** system SHALL throw an error with message describing the style validation failure

### Requirement: Column class preserved unchanged

The existing `Column` class (constructor, getters, setters) SHALL remain unchanged. It continues to be returned from column getters and used for programmatic column access.

#### Scenario: Column getter still returns Column class

- **WHEN** caller reads `ws.columns` after calling `setColumns`
- **THEN** each returned element SHALL be a `Column` instance with working getter/setter methods

#### Scenario: Column constructor still works

- **WHEN** caller writes `new Column("H", "k", 10)`
- **THEN** the system SHALL create a Column instance with header="H", key="k", width=10
