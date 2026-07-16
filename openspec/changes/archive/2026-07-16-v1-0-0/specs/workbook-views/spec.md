## ADDED Requirements

### Requirement: Workbook exposes views

A `Workbook` SHALL expose a `views` getter returning the workbook view
descriptors (array of `{ xWindow, yWindow, windowWidth, windowHeight,
activeTab, firstSheet, visibility, minimized, showHorizontalScroll,
showVerticalScroll, tabRatio }`) and a setter accepting such an array.

#### Scenario: Set workbook views

- **WHEN** `wb.views = [{ xWindow: 0, yWindow: 0, windowWidth: 12000, windowHeight: 8000, activeTab: 0, visibility: "visible" }]`
- **THEN** `wb.views[0].visibility === "visible"` and `windowWidth === 12000`

### Requirement: Workbook exposes calc properties

A `Workbook` SHALL expose `calcProperties` (`fullCalcOnLoad`, `calcId`,
`calcMode`, `refFullCalc`, `iterate`, `iterateCount`, `iterateDelta`) readable
and writable.

#### Scenario: Set fullCalcOnLoad

- **WHEN** `wb.calcProperties = { fullCalcOnLoad: true, calcId: 124519 }`
- **THEN** `wb.calcProperties.fullCalcOnLoad === true` and `calcId === 124519`

### Requirement: Writer emits bookViews and calcPr

When a workbook has `views`, the writer SHALL emit `<bookViews><workbookView
.../></bookViews>` in `xl/workbook.xml` (after `sheets`, before `definedNames`).
When `calcProperties` is set, the writer SHALL emit `<calcPr .../>` in
`xl/workbook.xml`. Defaults SHALL be omitted when not explicitly set.

#### Scenario: Emit bookViews and calcPr

- **WHEN** `wb.views = [{ activeTab: 1, visibility: "visible" }]` and `wb.calcProperties = { fullCalcOnLoad: true }`
- **THEN** `xl/workbook.xml` contains `<bookViews><workbookView activeTab="1" visibility="visible"/></bookViews>` and `<calcPr fullCalcOnLoad="1"/>`

#### Scenario: No views omits bookViews

- **WHEN** a workbook has no `views` set
- **THEN** `xl/workbook.xml` SHALL NOT contain a `<bookViews>` element

### Requirement: Reader parses bookViews and calcPr

The reader SHALL parse `<bookViews><workbookView>` and `<calcPr>` from
`xl/workbook.xml` into `wb.views` and `wb.calcProperties`, preserving
attributes. A workbook without `<bookViews>` SHALL leave `wb.views` as a
sensible default without erroring.

#### Scenario: Read bookViews and calcPr from Excel/ExcelJS

- **WHEN** `xl/workbook.xml` contains `<bookViews><workbookView activeTab="2" minimized="1"/></bookViews>` and `<calcPr fullCalcOnLoad="1" calcId="0"/>`
- **THEN** `wb.views[0].activeTab === 2`, `wb.views[0].minimized === true`, and `wb.calcProperties.fullCalcOnLoad === true`
