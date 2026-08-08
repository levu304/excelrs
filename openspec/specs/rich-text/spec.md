# rich-text Specification

## Purpose

Rich-text cell content round-trip: `CellValue.rich_text` runs with per-run `Font`, parsed on read and emitted on write. Write has shipped since v0.5.0; v0.12.0 adds the read side.

## Requirements

### Requirement: Reader parses rich-text cell content (theme/val-aware fonts)

When reading an `.xlsx`, the reader SHALL parse rich-text cell values (inline `<is><r>` and shared-string `<si><r>` runs) into a `CellValue` with `value_type === "RichText"` and `rich_text` equal to the ordered `Vec<RichTextRun>`, where each run carries its `text` and per-run `Font` whose attributes are resolved per run-level `<rPr>`:

- **name** from `<rFont val="N"/>`. When a run's `<rPr>` contains no `<rFont>` element, `font.name` SHALL be `null` — the reader SHALL NOT inject the default font name (`"Calibri"`); the run inherits the cell's default font on render.
- **size** from `<sz val="P"/>` (points). When `<rPr>` contains no `<sz>`, `font.size` SHALL be `null`.
- **bold/italic/underline** from `<b/>`/`<i/>`/`<u/>` honoring `val`: `val` absent or `"1"`/`"true"` ⇒ `Some(true)`; `val` `"0"`/`"false"`/`"none"` ⇒ `Some(false)`; `<u val="double"/>` ⇒ `Some(true)` (double not distinguishable in bool field).
- **color** resolved from `<color>` to a single ARGB hex string (`FFRRGGBB`) on `Font.color`: `rgb` attribute used directly; `theme="N"` resolved via the workbook `xl/theme/theme1.xml` scheme (slot + optional `tint`); `indexed="N"` resolved via the workbook color palette; `auto` ⇒ `"FF000000"`.

#### Scenario: Read rich text written by excelrs

- **WHEN** a cell was written with `cell.value = { richText: [{ text: "Hello ", font: { bold: true } }, { text: "World" }] }`, the workbook is written and read back
- **THEN** `cell.value.value_type === "RichText"` and `cell.value.rich_text` equals the two runs with the bold flag preserved on the first run

#### Scenario: Plain string cell is not rich text

- **WHEN** a cell holds a plain string value
- **THEN** `cell.value.value_type === "String"` and `rich_text` is `undefined`/`null`

#### Scenario: Read rich text from shared strings (Excel/ExcelJS output)

- **WHEN** a workbook stores rich-text runs as shared strings (`<si><r><rPr><rFont/></rPr><t></t></r></si>` in `xl/sharedStrings.xml`) referenced by `<c t="s"><v>idx</v></c>` cells
- **THEN** the reader SHALL resolve the shared-string index, find the rich-text runs, and return `cell.value.type === "RichText"` with `cell.richText` containing the runs and per-run fonts preserved

#### Scenario: Read rich text preserves font from shared strings

- **WHEN** a shared-strings rich-text cell has runs with distinct fonts (e.g., run 1: name=Arial size=12 bold; run 2: name=Times New Roman size=10 color=FF0000FF)
- **THEN** `cell.richText[0].font.name === "Arial"`, `cell.richText[0].font.size ≈ 12`, `cell.richText[0].font.bold === true`, `cell.richText[1].font.color === "FF0000FF"`

#### Scenario: Rich-text run honors `val="0"` to turn bold/italic off

- **WHEN** a run's `<rPr>` is `<b val="0"/><i/><u val="none"/>`
- **THEN** `font.bold === Some(false)`, `font.italic === Some(true)`, `font.underline === Some(false)`

#### Scenario: Rich-text run resolves theme color + tint to ARGB

- **WHEN** a shared-strings run font is `<color theme="4" tint="0.5"/>` and the workbook `xl/theme/theme1.xml` defines `accent1` = `4F81BD`
- **THEN** `font.color` equals the theme resolver's ARGB for accent1 lightened by tint 0.5 (`FFA7C0DE` for the OOXML default accent1 `4F81BD`); the result is a non-null `FFRRGGBB` string. If `theme1.xml` is absent, the default scheme's `4F81BD` is used (same resolver); if a custom `theme1.xml` defines a different `accent1`, that value is resolved instead.

#### Scenario: Rich-text run resolves indexed / auto color to ARGB

- **WHEN** a run font uses `<color indexed="8"/>` and another uses `<color auto="1"/>`
- **THEN** `font.color` is a non-null ARGB string derived from the workbook indexed palette (or OOXML system default) for the indexed run, and `font.color === "FF000000"` for the auto run

#### Scenario: Run without `<rFont>` inherits no default font name

- **WHEN** a rich-text run's `<rPr>` contains formatting (e.g. `<b/>` or `<sz val="12"/>`) but no `<rFont>` element
- **THEN** `font.name` SHALL be `null` (not `"Calibri"`), and `font.size` SHALL be `null` when no `<sz>` is present; the run inherits the cell's default font and size on render

#### Scenario: Run with `<rFont>` keeps its name

- **WHEN** a rich-text run's `<rPr>` contains `<rFont val="Arial"/>`
- **THEN** `font.name` SHALL equal `"Arial"` (unchanged from prior behavior)

### Requirement: Cell.richText accessor returns runs without a cast

The `Cell.richText` getter (`#[napi(getter)]`) SHALL return `RichTextRun[] | null` — the parsed runs when the cell is a RichText cell, or `null` otherwise. The caller SHALL read `cell.richText` directly (property access, no parentheses) without any `as CellValue` cast.

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
### Requirement: Writer emits rich text via shared strings for cross-app compatibility

When writing a rich-text cell (`cell.value.value_type === "RichText"`), the writer SHALL serialize the runs into the shared-string table (`xl/sharedStrings.xml`) as `<si><r><rPr>…</rPr><t>…</t></r></si>` and emit the cell as `t="s"` with the shared-string index in `<v>…</v>`. The writer SHALL NOT emit rich text as inline strings (`t="inlineStr"`).

This requirement exists because Apple Numbers' XLSX importer ignores inline-string rich-text run fonts and falls back to the cell's default font (Calibri); shared-string rich text is imported correctly by Numbers, Excel, and LibreOffice. The writer output remains schema-valid OOXML either way — the change is a compatibility improvement, not a correctness fix.

#### Scenario: Rich text written as shared string

- **WHEN** a rich-text cell with a run carrying `font.name === "Times New Roman"` is written
- **THEN** the cell element SHALL use `t="s"` with `<v>idx</v>` (not `t="inlineStr"`)
- **AND** `xl/sharedStrings.xml` SHALL contain an `<si>` whose `<r>` carries `<rPr><rFont val="Times New Roman"/></rPr>` and the run text in `<t>`

#### Scenario: Numbers renders the specified per-run fonts

- **WHEN** the written workbook is opened in Apple Numbers
- **THEN** the rich-text runs SHALL render in their specified per-run fonts (e.g. "Times New Roman"), not the Calibri default

#### Scenario: Round-trip preserves runs

- **WHEN** a rich-text cell is written and the file is read back
- **THEN** `cell.value.rich_text` SHALL match the written runs (ordered `text` and per-run `font` preserved)

#### Scenario: Identical rich text deduplicates to one shared string

- **WHEN** two cells contain rich text with identical runs (same text and per-run fonts)
- **THEN** both cells SHALL reference the same shared-string index

### Requirement: Rich-text run text preserves significant whitespace

When writing a rich-text run whose text contains leading/trailing whitespace or newlines (e.g. `"B: (11) = (7) + (10)\n"`), the writer SHALL emit the run's `<t>` element with `xml:space="preserve"` so the whitespace and newlines are not collapsed by the consumer.

#### Scenario: Trailing newline preserved

- **WHEN** a rich-text run text is `"B: (11) = (7) + (10)\n"`
- **THEN** the emitted `<t>` element SHALL carry `xml:space="preserve"` and the newline SHALL be retained in the output

