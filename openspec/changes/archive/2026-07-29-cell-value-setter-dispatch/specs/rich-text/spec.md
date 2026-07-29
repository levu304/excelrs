## ADDED Requirements

### Requirement: Public setter accepts rich-text object

The public `Cell.value` setter SHALL accept a rich-text object so the rich-text capability is reachable from JavaScript.

#### Scenario: Write rich text through the public setter and read back

- **WHEN** a cell is written via `cell.value = { richText: [{ text: "Hello ", font: { bold: true } }, { text: "World" }] }`, the workbook is saved and read back
- **THEN** `cell.value.value_type` SHALL be `"RichText"` and `cell.value.rich_text` SHALL equal the two runs with the bold flag preserved on the first run
