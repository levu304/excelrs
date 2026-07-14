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

### Requirement: Workbook round-trips rich text

A workbook written by excelrs with rich-text runs SHALL, after being read back, yield a `CellValue` whose `rich_text` runs match the originally written runs (text and per-run font).

#### Scenario: Write then read preserves runs

- **WHEN** rich text is written and the file is read back
- **THEN** the run count, each run's `text`, and each run's `font` match the written values
