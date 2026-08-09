## Purpose

Ensures the reader maps each worksheet to its real XML file via the workbook's relationship graph, not by positional sheet-file numbering.

## ADDED Requirements

### Requirement: Resolve worksheet file by relationship

The reader SHALL determine the XML file for each worksheet from the `xl/workbook.xml.rels` relationship (`rId → target`) combined with the `<sheet r:id>` order in `xl/workbook.xml`, rather than assuming `worksheets/sheetN.xml` matches display order.

#### Scenario: Workbook with reordered sheet files

- **WHEN** a workbook lists sheets A, B, C in display order but their files are sheet1.xml (A), sheet3.xml (B), sheet2.xml (C)
- **THEN** worksheet A reads from sheet1.xml, B from sheet3.xml, and C from sheet2.xml

#### Scenario: Standard file order

- **WHEN** a workbook's sheet files are numbered in display order (the common case)
- **THEN** resolution matches the previous positional behavior (no regression)

### Requirement: All per-sheet parsers use resolved path

The reader SHALL apply the relationship-resolved path to every per-sheet parser (state, tabColor/defaults, data validations, conditional formatting, merged cells, etc.) so no parser attaches data to the wrong worksheet.

#### Scenario: tabColor on a reordered sheet

- **WHEN** sheet B has a tabColor stored in sheet3.xml and the files are reordered as above
- **THEN** B's tabColor is read from sheet3.xml and attached to worksheet B

#### Scenario: No data loss across parsers

- **WHEN** a reordered workbook is read
- **THEN** every per-sheet parser attaches its data to the correct worksheet, not the positionally-indexed one

### Requirement: Missing relationships degrade safely

The reader SHALL fall back to positional `sheet{i+1}.xml` indexing when `xl/workbook.xml.rels` or the `<sheet r:id>` attributes are absent, preserving current behavior for malformed or minimal workbooks.

#### Scenario: Workbook without rels

- **WHEN** a workbook lacks `xl/workbook.xml.rels` or `<sheet r:id>` attributes
- **THEN** the reader uses positional indexing as before (no panic, no data loss)
