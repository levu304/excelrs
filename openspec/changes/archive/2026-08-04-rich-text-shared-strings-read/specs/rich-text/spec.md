## MODIFIED Requirements

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
