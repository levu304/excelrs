# rich-text Specification

## MODIFIED Requirements

### Requirement: Reader parses rich-text cell content (theme/val-aware fonts)

When reading an `.xlsx`, reader SHALL parse rich-text cell values (inline
`<is><r>` and shared-string `<si><r>` runs) into `CellValue`
`value_type === "RichText"` with `rich_text` an ordered `Vec<RichTextRun>`,
each run carrying its `text` and per-run `Font` attributes resolved from the
run-level `<rPr>`:

- **name** from `<rFont val="N"/>`. When a run's `<rPr>` contains no `<rFont>`
  element, `font.name` SHALL be `null` — the reader SHALL NOT inject the
  default font name (`"Calibri"`). The run inherits the cell's default font on
  render.
- **size** from `<sz val="P"/>` (points). When `<rPr>` contains no `<sz>`,
  `font.size` SHALL be `null`.
- **bold/italic/underline** from `<b/>`/`<i/>`/`<u/>` honoring `val`: `val`
  absent or `"1"`/`"true"` ⇒ `Some(true)`; `val` `"0"`/`"false"`/`"none"` ⇒
  `Some(false)`; `<u val="double"/>` ⇒ `Some(true)` (double not distinguishable
  in bool field).
- **color** resolved from `<color>` into a single ARGB hex string (`FFRRGGBB`)
  on `Font.color`: `rgb` attribute used directly; `theme="N"` resolved via the
  workbook `xl/theme/theme1.xml` scheme (slot + optional `tint`); `indexed="N"`
  resolved via the workbook color palette; `auto` ⇒ `"FF000000"`.

#### Scenario: Run without `<rFont>` inherits no default font name

- **WHEN** a rich-text run's `<rPr>` contains formatting (e.g. `<b/>` or
  `<sz val="12"/>`) but no `<rFont>` element
- **THEN** `font.name` SHALL be `null` (not `"Calibri"`), and `font.size` SHALL
  be `null` when no `<sz>` is present; the run inherits the cell's default font
  and size on render

#### Scenario: Run with `<rFont>` keeps its name

- **WHEN** a rich-text run's `<rPr>` contains `<rFont val="Arial"/>`
- **THEN** `font.name` SHALL equal `"Arial"` (unchanged from prior behavior)
