## Purpose

Preserves worksheet-level metadata (sheet visibility, tab color, and default row/column
dimensions) across read→write round-trips and exposes it through an ExcelJS-compatible API.

## ADDED Requirements

### Requirement: Sheet visibility state is preserved

The system SHALL read and write a worksheet's visibility state (`visible`, `hidden`, or
`veryHidden`) and SHALL preserve it across a read→write round-trip. The default state
(`visible`) SHALL be emitted without a `state` attribute to match ExcelJS output.

#### Scenario: Reading a hidden sheet preserves its hidden state

- **WHEN** a workbook containing a `hidden` sheet is read
- **THEN** `ws.state` returns `'hidden'` for that worksheet

#### Scenario: Reading a very-hidden sheet preserves its state

- **WHEN** a workbook containing a `veryHidden` sheet is read
- **THEN** `ws.state` returns `'veryHidden'` for that worksheet

#### Scenario: Hidden state survives a round-trip

- **WHEN** a workbook with a hidden sheet is read and written back
- **THEN** the written `xl/workbook.xml` contains `<sheet ... state="hidden"/>` for that sheet

#### Scenario: Setting state writes the attribute

- **WHEN** `ws.state` is set to `'hidden'` before writing
- **THEN** the emitted `<sheet>` element carries `state="hidden"`

#### Scenario: Visible sheets omit the state attribute

- **WHEN** `ws.state` is `'visible'` (the default)
- **THEN** the emitted `<sheet>` element has no `state` attribute

### Requirement: Tab color is preserved

The system SHALL read and write a worksheet's tab color as a `Color` (supporting ARGB,
theme, and indexed variants) and SHALL preserve it across a read→write round-trip.

#### Scenario: Reading a tab color returns a Color

- **WHEN** a workbook with a colored tab is read
- **THEN** `ws.properties.tabColor` returns a `Color` matching the source

#### Scenario: Tab color survives a round-trip

- **WHEN** a workbook with a tab color is read and written back
- **THEN** the written `<worksheet>` contains `<sheetPr><tabColor .../></sheetPr>` with the
  same color value

#### Scenario: Setting the tab color emits sheetPr

- **WHEN** `ws.properties.tabColor` is set before writing
- **THEN** `sheetPr` is emitted as the first child of `<worksheet>` with the tab color

#### Scenario: No tab color emits no sheetPr

- **WHEN** a worksheet has no tab color set
- **THEN** no `<sheetPr>` element is emitted

### Requirement: Default row and column dimensions are preserved

The system SHALL read and write default row height, default column width, and the
worksheet-level row/column outline levels, and SHALL preserve them across a read→write
round-trip.

#### Scenario: Reading default row height

- **WHEN** a workbook with `<sheetFormatPr defaultRowHeight="24"/>` is read
- **THEN** `ws.properties.defaultRowHeight` returns `24`

#### Scenario: Default row height survives a round-trip

- **WHEN** a workbook with `defaultRowHeight="24"` is read and written back
- **THEN** the written worksheet contains `<sheetFormatPr defaultRowHeight="24"/>`

#### Scenario: Default column width survives a round-trip

- **WHEN** a workbook with `defaultColWidth="20"` is read and written back
- **THEN** the written worksheet contains `<sheetFormatPr defaultColWidth="20"/>`

#### Scenario: Outline levels survive a round-trip

- **WHEN** a workbook with `outlineLevelRow="2"` and `outlineLevelCol="1"` is read and
  written back
- **THEN** the written `<sheetFormatPr>` carries `outlineLevelRow="2"` and
  `outlineLevelCol="1"`

#### Scenario: Unset dimensions emit no sheetFormatPr

- **WHEN** a worksheet has no default row/column dimensions set
- **THEN** no `<sheetFormatPr>` element is emitted

### Requirement: AddWorksheetOptions accepts state and properties

The system SHALL accept `state` and `properties` in `Workbook.addWorksheet(name, options)`
so a worksheet can be created with visibility and tab color set up front.

#### Scenario: Creating a hidden sheet via options

- **WHEN** `addWorksheet('Secret', { state: 'hidden', properties: { tabColor } })` is called
- **THEN** the created worksheet has `state === 'hidden'` and the given `tabColor`

#### Scenario: Options omitted default to visible

- **WHEN** `addWorksheet('Sheet1')` is called with no options
- **THEN** the worksheet defaults to `state === 'visible'` with no tab color
