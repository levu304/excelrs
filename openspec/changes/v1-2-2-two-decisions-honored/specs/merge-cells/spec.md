## ADDED Requirements

### Requirement: Merged cell ranges survive a write to read round-trip

The reader SHALL parse `<mergeCells>` from the worksheet XML and restore the merged ranges so a workbook written with merged cells reads back with the same merges.

#### Scenario: Write then read a merged range

- **WHEN** a workbook has `ws.mergeCells("B2:D4")` applied, is written, and is read back
- **THEN** the read-back worksheet SHALL report the same merged range (`B2:D4`)

#### Scenario: Anchor cell keeps its value, non-master cells stay empty

- **WHEN** the top-left anchor cell of a merge holds a value and the workbook is written then read back
- **THEN** the read-back anchor cell SHALL retain that value and the other cells inside the range SHALL carry no phantom value
