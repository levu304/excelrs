# rich-text Specification

## Purpose

Rich-text cell content round-trip: `CellValue.rich_text` runs with per-run `Font`, parsed on read and emitted on write. Write has shipped since v0.5.0; v0.12.0 adds the read side.

## Requirements

### Requirement: Reader parses rich-text cell content

When reading an `.xlsx`, the reader SHALL parse rich-text cell values (inline `<is><r>` and shared-string `<si><r>` runs) into a `CellValue` with `value_type === "RichText"` and `rich_text` equal to the ordered `Vec<RichTextRun>`, where each run carries its `text` and per-run `Font` (name/size/bold/italic/underline/color).

#### Scenario: Read rich text written by excelrs

- **WHEN** a cell was written with `cell.value = { richText: [{ text: "Hello ", font: { bold: true } }, { text: "World" }] }`, the workbook is written and read back
- **THEN** `cell.value.value_type === "RichText"` and `cell.value.rich_text` equals the two runs with the bold flag preserved on the first run

#### Scenario: Plain string cell is not rich text

- **WHEN** a cell holds a plain string value
- **THEN** `cell.value.value_type === "String"` and `rich_text` is `undefined`/`null`

#### Scenario: Read rich text from shared strings (Excel/ExcelJS output)

- **WHEN** a workbook stores rich-text runs as shared strings (`<si><r><rPr><rFont/></rPr><t></t></r></si>` in `xl/sharedStrings.xml`) referenced by `<c t="s"><v>idx</v></c>` cells
- **THEN** the reader SHALL resolve the shared-string index, find the rich-text runs, and return `cell.value.type === "RichText"` with `cell.richText` containing the runs and per-run fonts (name, size, bold, italic, underline, color) preserved

#### Scenario: Read rich text preserves font from shared strings

- **WHEN** a shared-strings rich-text cell has runs with distinct fonts (e.g., run 1: name=Arial size=12 bold; run 2: name=Times New Roman size=10 color=FF0000FF)
- **THEN** `cell.richText[0].font.name === "Arial"`, `cell.richText[0].font.size ≈ 12`, `cell.richText[0].font.bold === true`, `cell.richText[1].font.color === "FF0000FF"`

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

### Requirement: Workbook round-trips rich text

A workbook written by excelrs with rich-text runs SHALL, after being read back, yield a `CellValue` whose `rich_text` runs match the originally written runs (text and per-run font).

#### Scenario: Write then read preserves runs

- **WHEN** rich text is written and the file is read back
- **THEN** the run count, each run's `text`, and each run's `font` match the written values

### Requirement: Public setter accepts rich-text object

The public `Cell.value` setter SHALL accept a rich-text object so the rich-text capability is reachable from JavaScript.

#### Scenario: Write rich text through the public setter and read back

- **WHEN** a cell is written via `cell.value = { richText: [{ text: "Hello ", font: { bold: true } }, { text: "World" }] }`, the workbook is saved and read back
- **THEN** `cell.value.value_type` SHALL be `"RichText"` and `cell.value.rich_text` SHALL equal the two runs with the bold flag preserved on the first run
