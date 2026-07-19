## ADDED Requirements

### Requirement: Streaming reader resolves sheet files from rels targets tolerating absolute paths

The streaming reader SHALL resolve each worksheet's XML part from the targets in
`xl/_rels/workbook.xml.rels`. It SHALL tolerate both relative targets (relative to
`xl/`) and absolute, package-rooted targets (leading `/`), resolving each to the
correct package path without a doubled `xl/` prefix.

#### Scenario: Absolute rels Target resolves to its package path

- **WHEN** a workbook's `xl/_rels/workbook.xml.rels` declares `Target="/xl/worksheets/sheet1.xml"`
- **THEN** the reader resolves the sheet at package path `xl/worksheets/sheet1.xml` (not `xl//xl/worksheets/sheet1.xml`) and reads its rows

#### Scenario: Relative rels Target resolves as before

- **WHEN** a rels `Target` is `worksheets/sheet1.xml`
- **THEN** the reader resolves `xl/worksheets/sheet1.xml` and reads its rows
