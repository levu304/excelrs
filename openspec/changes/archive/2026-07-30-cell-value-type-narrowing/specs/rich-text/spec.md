## ADDED Requirements

### Requirement: Cell.richText accessor returns runs without a cast

The `Cell.richText` getter (`#[napi(getter)]`) SHALL return `RichTextRun[] | null` — the
parsed runs when the cell is a RichText cell, or `null` otherwise. The caller SHALL read
`cell.richText` directly (property access, no parentheses) without any `as CellValue` cast.

#### Scenario: Read runs directly

- **WHEN** a RichText cell was written with two runs and `cell.type === "RichText"`
- **THEN** `cell.richText` SHALL be the `RichTextRun[]` (length 2) with the bold flag preserved on the first run, typed without any cast

#### Scenario: Non-RichText cell returns null

- **WHEN** a Number cell stores `42`
- **THEN** `cell.richText` SHALL be `null`
