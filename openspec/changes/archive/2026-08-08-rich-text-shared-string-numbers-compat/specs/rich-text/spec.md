## ADDED Requirements

### Requirement: Writer emits rich text via shared strings for cross-app compatibility

When writing a rich-text cell (`cell.value.value_type === "RichText"`), the
writer SHALL serialize the runs into the shared-string table
(`xl/sharedStrings.xml`) as `<si><r><rPr>…</rPr><t>…</t></r></si>` and emit the
cell as `t="s"` with the shared-string index in `<v>…</v>`. The writer SHALL NOT
emit rich text as inline strings (`t="inlineStr"`).

This requirement exists because Apple Numbers' XLSX importer ignores
inline-string rich-text run fonts and falls back to the cell's default font
(Calibri); shared-string rich text is imported correctly by Numbers, Excel, and
LibreOffice. The writer output MUST remain schema-valid OOXML either way — the
change is a compatibility improvement, not a correctness fix.

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

When writing a rich-text run whose text contains leading/trailing whitespace or
newlines (e.g. `"B: (11) = (7) + (10)\n"`), the writer SHALL emit the run's `<t>`
element with `xml:space="preserve"` so the whitespace and newlines are not
collapsed by the consumer.

#### Scenario: Trailing newline preserved

- **WHEN** a rich-text run text is `"B: (11) = (7) + (10)\n"`
- **THEN** the emitted `<t>` element SHALL carry `xml:space="preserve"` and the newline SHALL be retained in the output
