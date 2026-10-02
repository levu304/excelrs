# rich-text Specification

## Purpose

Rich-text cell content round-trip: `CellValue.rich_text` runs with per-run `Font`, parsed on read and emitted on write. Write has shipped since v0.5.0; v0.12.0 adds the read side.

## Requirements

### Requirement: Reader parses rich-text runs into ordered RichText cell values

When reading an `.xlsx`, the reader SHALL parse rich-text cell values — inline `<is><r>` and
shared-string `<si><r>` runs — into a `CellValue` with `value_type === "RichText"` and
`rich_text` equal to the ordered runs, where each run carries its `text` and a per-run
`Font` resolved from the run's own `<rPr>`.

#### Scenario: Inline rich text parses into ordered runs

- **WHEN** the reader parses a cell containing an inline `<is><r>` run
- **THEN** the cell SHALL yield `value_type === "RichText"` with the run in `rich_text` in document order

#### Scenario: Shared-string rich text parses into ordered runs

- **WHEN** the reader parses a cell whose shared string contains an `<si><r>` run
- **THEN** the cell SHALL yield `value_type === "RichText"` with the run in `rich_text` in document order

### Requirement: Reader resolves run font name from rFont, defaulting to null

`Font.name` SHALL be resolved from `<rFont val="N"/>`. When a run's `<rPr>` contains no
`<rFont>` element, `font.name` SHALL be `null`; the reader SHALL NOT inject the default font
name (`"Calibri"`), because the run inherits the cell's default font on render.

#### Scenario: rFont supplies the run font name

- **WHEN** a run's `<rPr>` contains `<rFont val="Arial"/>`
- **THEN** `font.name` SHALL be `"Arial"`

#### Scenario: Missing rFont yields null

- **WHEN** a run's `<rPr>` contains no `<rFont>` element
- **THEN** `font.name` SHALL be `null` and SHALL NOT be the default font name

### Requirement: Reader resolves run font size from sz, defaulting to null

`Font.size` SHALL be resolved from `<sz val="P"/>` in points. When `<rPr>` contains no
`<sz>`, `font.size` SHALL be `null`.

#### Scenario: sz supplies the run font size

- **WHEN** a run's `<rPr>` contains `<sz val="14"/>`
- **THEN** `font.size` SHALL be `14`

#### Scenario: Missing sz yields null

- **WHEN** a run's `<rPr>` contains no `<sz>` element
- **THEN** `font.size` SHALL be `null`

### Requirement: Reader resolves bold, italic, and underline honoring val

`bold`, `italic`, and `underline` SHALL be resolved from `<b/>`, `<i/>`, and `<u/>` honoring
the `val` attribute: absent or `"1"` / `"true"` yields `Some(true)`, while `"0"` / `"false"`
/ `"none"` yields `Some(false)`. `<u val="double"/>` yields `Some(true)`, because the double
style is not distinguishable in the boolean field.

#### Scenario: Bare toggle element means true

- **WHEN** a run's `<rPr>` contains `<b/>` with no `val`
- **THEN** `font.bold` SHALL be `Some(true)`

#### Scenario: Explicitly false val means false

- **WHEN** a run's `<rPr>` contains `<i val="0"/>`
- **THEN** `font.italic` SHALL be `Some(false)`

#### Scenario: Underline none means false

- **WHEN** a run's `<rPr>` contains `<u val="none"/>`
- **THEN** `font.underline` SHALL be `Some(false)`

#### Scenario: Underline double collapses to true

- **WHEN** a run's `<rPr>` contains `<u val="double"/>`
- **THEN** `font.underline` SHALL be `Some(true)`

### Requirement: Reader resolves run color from rgb, theme, indexed, and auto

`Font.color` SHALL be resolved from `<color>` to a single ARGB hex string (`FFRRGGBB`): the
`rgb` attribute used directly, `theme="N"` resolved via the workbook `xl/theme/theme1.xml`
scheme with any `tint`, `indexed="N"` resolved via the workbook color palette, and `auto`
yielding `"FF000000"`.

#### Scenario: rgb attribute is used directly

- **WHEN** a run's color element carries `rgb="FF112233"`
- **THEN** `font.color` SHALL be `"FF112233"`

#### Scenario: theme color resolves through the theme scheme

- **WHEN** a run's color element carries `theme="4"` with a `tint`
- **THEN** `font.color` SHALL be the ARGB hex resolved from theme slot 4 with the tint applied

#### Scenario: indexed color resolves through the workbook palette

- **WHEN** a run's color element carries `indexed="12"`
- **THEN** `font.color` SHALL be the ARGB hex for index 12 in the workbook color palette

#### Scenario: auto color yields black

- **WHEN** a run's color element carries `auto="1"`
- **THEN** `font.color` SHALL be `"FF000000"`

### Requirement: Writer emits rich text through the shared-strings table

When writing a rich-text cell (`cell.value.value_type === "RichText"`), the writer SHALL
serialize the runs into the shared-string table (`xl/sharedStrings.xml`) as
`<si><r><rPr>…</rPr><t>…</t></r></si>` and emit the cell as `t="s"` with the shared-string
index in `<v>…</v>`.

#### Scenario: Rich-text cell emits a shared-string reference

- **WHEN** a rich-text cell is written
- **THEN** the run SHALL be serialized into `xl/sharedStrings.xml` as an `<si><r>` entry and the cell SHALL be emitted as `t="s"` with that index in `<v>`

#### Scenario: Run properties and text survive serialization

- **WHEN** a rich-text run carrying `<rPr>` properties and text is written
- **THEN** the emitted `<si><r>` SHALL carry `<rPr>…</rPr><t>…</t>`

### Requirement: Rich text uses shared strings for cross-app compatibility

The writer SHALL emit rich text through the shared-strings table because Apple Numbers' XLSX
importer ignores inline-string rich-text run fonts and falls back to the cell's default font
(Calibri). Shared-string rich text SHALL be imported correctly by Numbers, Excel, and
LibreOffice. The writer output remains schema-valid OOXML either way — the shared-strings
path is a compatibility improvement, not a correctness fix.

#### Scenario: Rich text is importable by strict consumers

- **WHEN** the workbook is opened in Apple Numbers
- **THEN** the run font SHALL be honored rather than falling back to the cell's default font

#### Scenario: Shared-strings path does not affect schema validity

- **WHEN** the workbook is validated against the OOXML schema
- **THEN** the shared-strings output SHALL be schema-valid

#### Scenario: Rich-text cell emits a shared-string reference

- **WHEN** a rich-text cell is written
- **THEN** the run SHALL be serialized into `xl/sharedStrings.xml` as an `<si><r>` entry and the cell SHALL be emitted as `t="s"` with that index in `<v>`

#### Scenario: Rich text is importable by strict consumers

- **WHEN** the workbook is opened in Apple Numbers
- **THEN** the run font SHALL be honored rather than falling back to the cell's default font

### Requirement: Writer does not emit rich text as inline strings

The writer SHALL NOT emit rich text as inline strings (`t="inlineStr"`).

#### Scenario: Rich-text cell never uses inlineStr

- **WHEN** a rich-text cell is written
- **THEN** the emitted cell SHALL NOT use `t="inlineStr"`

### Requirement: Rich-text writer output is covered by a golden-file test

The rich-text shared-string writer SHALL be covered by a golden-file test asserting the
exact emitted `xl/sharedStrings.xml` and `xl/worksheets/sheetN.xml` for a known rich-text
cell, including run `<rPr>` contents, `xml:space="preserve"`, and `t="s"` with `<v>`.

#### Scenario: Golden-file test pins the emitted shared strings

- **WHEN** the golden-file test runs for a known rich-text cell
- **THEN** it SHALL assert the exact `xl/sharedStrings.xml` and `xl/worksheets/sheetN.xml`, including run `<rPr>` contents, `xml:space="preserve"`, and `t="s"` with `<v>`

### Requirement: Rich-text writer output is covered by an OOXML conformance smoke test

The rich-text shared-string writer SHALL be covered by an OOXML conformance smoke test
validating the generated workbook against the OOXML schema and/or opening it headless in
LibreOffice.

#### Scenario: OOXML conformance smoke test runs

- **WHEN** the conformance smoke test runs
- **THEN** it SHALL validate the generated workbook against the OOXML schema and/or open it headless in LibreOffice

### Requirement: Manual Apple Numbers verification stays documented

A manual open in Apple Numbers via `scripts/rich-text-repro.cjs` SHALL remain a documented
verification step.

#### Scenario: Manual Numbers verification stays documented

- **WHEN** a developer verifies rich-text compatibility manually
- **THEN** `scripts/rich-text-repro.cjs` SHALL remain the documented path for opening the workbook in Apple Numbers

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

### Requirement: Rich-text run text preserves significant whitespace

When writing a rich-text run whose text contains leading/trailing whitespace or newlines (e.g. `"B: (11) = (7) + (10)\n"`), the writer SHALL emit the run's `<t>` element with `xml:space="preserve"` so the whitespace and newlines are not collapsed by the consumer.

#### Scenario: Trailing newline preserved

- **WHEN** a rich-text run text is `"B: (11) = (7) + (10)\n"`
- **THEN** the emitted `<t>` element SHALL carry `xml:space="preserve"` and the newline SHALL be retained in the output

### Requirement: Rich-text dedup key uses rendered fields only

When computing the shared-string dedup key for a rich-text cell, the writer
SHALL derive the key from the *rendered* run projection only — the fields
`write_rich_run_xml` actually emits: `name`, `size`, `bold`, `italic`,
`underline`, `color`. The writer SHALL NOT include unrendered fields
(`color_theme`, `color_tint`) in the key. Two runs that render identically SHALL
share one shared-string entry even if they differ only in unrendered fields.

#### Scenario: Theme/tint-only differences collapse to one entry

- **WHEN** two rich-text runs have identical rendered fields but differ only in
  `color_theme` / `color_tint` (e.g. both render as `Arial`/`FF0000FF`, one
  carries an internal theme link the other does not)
- **THEN** the writer SHALL assign them the same shared-string index (one
  `<si>` entry), not two.

#### Scenario: Dedup key is independent of validation

- **WHEN** the dedup key is built
- **THEN** its correctness SHALL NOT depend on `Font::validate` covering every
  keyed field; only emitted fields participate in the key.

### Requirement: Streaming writer emits rich text via shared strings

When the streaming writer supports rich-text write (a future `RichText` value
on the streaming path), it SHALL serialize runs into the shared-string table
as `<si><r><rPr>…</rPr><t>…</t></r></si>` and emit the cell as `t="s"` with the
shared-string index in `<v>…</v>`, reusing the same render/key logic as the
non-streaming writer. The streaming writer SHALL NOT emit rich text as inline
strings (`t="inlineStr"`).

#### Scenario: Streaming rich text uses shared strings, not inline

- **WHEN** a streaming rich-text cell is written
- **THEN** the cell element SHALL use `t="s"` with `<v>idx</v>` (not
  `t="inlineStr"`), and `xl/sharedStrings.xml` SHALL contain the run's `<r>`
  with its `<rPr>` and `<t>`.

#### Scenario: Streaming and non-streaming paths stay consistent

- **WHEN** the same rich-text content is written via the streaming and
  non-streaming writers
- **THEN** both SHALL produce shared-string rich text with identical rendering
  (no path emits inlineStr).
