# worksheet-views Specification

## Purpose

TBD - created by archiving change v0-11-0. Update Purpose after archive.

## Requirements

### Requirement: Worksheet exposes views (freeze/split)

A `Worksheet` SHALL expose a `views` getter returning the current view
descriptors (array of `{ state, xSplit, ySplit, topLeftCell, activePane, showGridLines }`) and
a setter accepting such an array. Each view SHALL capture `state` (`"frozen"` /
`"split"` / absent), `xSplit`/`ySplit` (pane split counts), `topLeftCell`, and
`activePane`. `showGridLines` SHALL be an optional boolean; when absent the
OOXML default of `true` applies.

#### Scenario: Set a frozen view

- **WHEN** `ws.views = [{ state: "frozen", xSplit: 1, ySplit: 2 }]`
- **THEN** `ws.views[0].state === "frozen"`, `xSplit === 1`, `ySplit === 2`

### Requirement: Writer emits sheetViews panes

When a worksheet has a view, the writer SHALL emit
`<sheetViews><sheetView state="{state}"{showGridLines}><pane xSplit=".." ySplit=".." topLeftCell=".." activePane=".."/></sheetView></sheetViews>`
at the CT_Worksheet schema position (immediately after `<dimension>`, before
`sheetProtection`/`autoFilter`/`mergeCells`). Omitted attributes SHALL be
emitted only when present. A worksheet with no views SHALL NOT emit
`<sheetViews>`.

#### Scenario: Emit a frozen pane

- **WHEN** a worksheet has `views = [{ state: "frozen", xSplit: 1, ySplit: 1 }]`
- **THEN** the sheet XML contains `<sheetViews><sheetView state="frozen"><pane xSplit="1" ySplit="1"/></sheetView></sheetViews>`

#### Scenario: Emit showGridLines="0"

- **WHEN** a sheet view has `showGridLines: false`
- **THEN** the sheet XML contains `<sheetView showGridLines="0" ...>`

#### Scenario: Emit showGridLines="1"

- **WHEN** a sheet view has `showGridLines: true`
- **THEN** the sheet XML contains `<sheetView showGridLines="1" ...>`

#### Scenario: Omit showGridLines when unset

- **WHEN** a sheet view has no `showGridLines` set
- **THEN** the `<sheetView>` element SHALL NOT contain a `showGridLines`
  attribute (reader and Excel apply the OOXML default of `true`)

#### Scenario: No views omits sheetViews

- **WHEN** a worksheet has no views set
- **THEN** the sheet XML SHALL NOT contain a `<sheetViews>` element

### Requirement: Reader parses sheetViews panes

The reader SHALL parse `<sheetViews><sheetView>` (and its `<pane>` child) from
`xl/worksheets/sheetN.xml` and populate `ws.views` with `state`, `xSplit`,
`ySplit`, `topLeftCell`, and `activePane` from the parsed attributes, plus
`showGridLines` from the `<sheetView>` attribute. A sheet
without `<sheetViews>` SHALL leave `ws.views` empty.

#### Scenario: Read a frozen pane written by Excel or ExcelJS

- **WHEN** a sheet XML contains `<sheetView state="frozen"><pane xSplit="2" ySplit="0"/></sheetView>`
- **THEN** `ws.views[0].state === "frozen"`, `xSplit === 2`, `ySplit === 0`

#### Scenario: File without sheetViews

- **WHEN** a workbook has no `<sheetViews>` in any sheet
- **THEN** `ws.views` is empty for every sheet; reader does not error

#### Scenario: Read showGridLines="0"

- **WHEN** a sheet XML contains `<sheetView showGridLines="0">`
- **THEN** `ws.views[0].showGridLines === false`

#### Scenario: Read showGridLines="1"

- **WHEN** a sheet XML contains `<sheetView showGridLines="1">`
- **THEN** `ws.views[0].showGridLines === true`

#### Scenario: Missing showGridLines is None

- **WHEN** a sheet XML contains `<sheetView>` without `showGridLines`
- **THEN** `ws.views[0].showGridLines` is `undefined` / `None`
