# hyperlinks Specification

## Purpose

Covers worksheet hyperlink support: reading `<hyperlinks>` from sheet XML (the
reader half added in v0.11.0) and the write/round-trip shape already shipping
since v0.5.0. A hyperlink cell carries a URL and a display text and must
survive a read → write → read cycle.

## ADDED Requirements

### Requirement: Reader parses worksheet hyperlinks

When reading an `.xlsx`, the reader SHALL parse `<hyperlinks>` from each
`xl/worksheets/sheetN.xml` and resolve each `<hyperlink r:id="rIdN">` to its
target URL via `xl/worksheets/_rels/sheetN.xml.rels`. For the cell at the
hyperlink `ref`, the reader SHALL set a `CellValue` with `value_type ===
"Hyperlink"`, `hyperlink` equal to the resolved URL, and `hyperlink_text`
equal to the cell's existing displayed string.

#### Scenario: Read a hyperlink written by ExcelJS or Excel

- **WHEN** a sheet has `<hyperlinks><hyperlink ref="B2" r:id="rId1"/></hyperlinks>` and `sheetN.xml.rels` maps `rId1` → `https://example.com`
- **THEN** `cell("B2").value` has `value_type === "Hyperlink"`, `hyperlink === "https://example.com"`, and `hyperlink_text` equals the displayed text of `B2`

#### Scenario: Sheet without hyperlinks

- **WHEN** a workbook has no `<hyperlinks>` in any sheet
- **THEN** no cell's `value_type` becomes `"Hyperlink"`; the reader does not error

### Requirement: Workbook round-trips hyperlinks

A workbook written by `excelrs` with a hyperlink cell SHALL, after being read
back, yield a `CellValue` whose `hyperlink` and `hyperlink_text` match the
originally written values.

#### Scenario: Write then read preserves the link

- **WHEN** `cell("A1").value = { text: "Site", hyperlink: "https://example.com" }`, the workbook is written and read back
- **THEN** `cell("A1").value.hyperlink === "https://example.com"` and `cell("A1").value.text === "Site"`
