## REMOVED Requirements

### Requirement: Reader parses rich-text cell content (theme/val-aware fonts)

**Reason**: A single requirement enumerated five per-run font attributes, each with its own element source, defaulting rule, and value-handling rules, producing 1,295 characters of prose.

**Migration**: Replaced by "Reader parses rich-text cell content (theme/val-aware fonts)" plus one requirement per font attribute, and "Reader resolves run color from rgb, theme, indexed, and auto".

### Requirement: Writer emits rich text via shared strings for cross-app compatibility

**Reason**: Bundled the shared-strings serialization contract with its rationale for the no-inline-strings rule into one over-long requirement.

**Migration**: Replaced by "Writer emits rich text via shared strings for cross-app compatibility" and "Writer does not emit rich text as inline strings".

### Requirement: Writer output is verifiable for cross-app compatibility

**Reason**: Combined the golden-file test, the OOXML conformance smoke test, and the manual Numbers verification step into one over-long requirement.

**Migration**: Replaced by "Writer output is verifiable for cross-app compatibility", which carries the three verification mechanisms as separate scenarios.

## ADDED Requirements

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
index in `<v>…</v>`. Apple Numbers' XLSX importer ignores inline-string rich-text run fonts
and falls back to the cell's default font (Calibri); shared-string rich text is imported
correctly by Numbers, Excel, and LibreOffice. The writer output remains schema-valid OOXML
either way — the shared-strings path is a compatibility improvement, not a correctness fix.

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

### Requirement: Rich-text writer output is covered by automated confidence checks

The rich-text shared-string writer SHALL be covered by automated confidence checks: a
golden-file test asserting the exact emitted `xl/sharedStrings.xml` and
`xl/worksheets/sheetN.xml` for a known rich-text cell (run `<rPr>` contents,
`xml:space="preserve"`, and `t="s"` with `<v>`); and an OOXML conformance smoke test
validating the generated workbook against the OOXML schema and/or opening it headless in
LibreOffice. A manual open in Apple Numbers via `scripts/rich-text-repro.cjs` SHALL remain a
documented verification step.

#### Scenario: Golden-file test pins the emitted shared strings

- **WHEN** the golden-file test runs for a known rich-text cell
- **THEN** it SHALL assert the exact `xl/sharedStrings.xml` and `xl/worksheets/sheetN.xml`, including run `<rPr>` contents, `xml:space="preserve"`, and `t="s"` with `<v>`

#### Scenario: OOXML conformance smoke test runs

- **WHEN** the conformance smoke test runs
- **THEN** it SHALL validate the generated workbook against the OOXML schema and/or open it headless in LibreOffice

#### Scenario: Manual Numbers verification stays documented

- **WHEN** a developer verifies rich-text compatibility manually
- **THEN** `scripts/rich-text-repro.cjs` SHALL remain the documented path for opening the workbook in Apple Numbers