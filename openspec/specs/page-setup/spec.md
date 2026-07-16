# page-setup Specification

## Purpose
TBD - created by archiving change v1-0-0. Update Purpose after archive.
## Requirements
### Requirement: Worksheet exposes pageSetup

A `Worksheet` SHALL expose a `pageSetup` getter returning the current page
setup descriptor (`orientation`, `paperSize`, `fitToPage`, `fitToWidth`,
`fitToHeight`, `horizontalDpi`, `verticalDpi`, `blackAndWhite`,
`drawingPrinted`, `cellComments`, `copies`) and a setter accepting such a
descriptor.

#### Scenario: Set page setup

- **WHEN** `ws.pageSetup = { orientation: "landscape", paperSize: 9, fitToPage: true, fitToWidth: 1 }`
- **THEN** `ws.pageSetup.orientation === "landscape"`, `paperSize === 9`, `fitToPage === true`, `fitToWidth === 1`

### Requirement: Worksheet exposes print margins

A `Worksheet` SHALL expose `pageSetup` margins (`top`, `bottom`, `left`,
`right`, `header`, `footer` in inches) readable and writable.

#### Scenario: Set margins

- **WHEN** `ws.pageSetup = { margins: { top: 0.5, bottom: 0.5, left: 0.25, right: 0.25, header: 0.3, footer: 0.3 } }`
- **THEN** the returned `pageSetup.margins` equals those values

### Requirement: Writer emits pageMargins and pageSetup

When a worksheet has a `pageSetup`, the writer SHALL emit `<pageMargins>`
(left/right/top/bottom/header/footer) and `<pageSetup>` (orientation,
paperSize, fitToPage, fitToWidth, fitToHeight, …) at the CT_Worksheet position
(after `headerFooter`, near sheet end). `printArea` and `printTitles` SHALL be
emitted as workbook-defined names `_xlnm.Print_Area` / `_xlnm.Print_Titles`.

#### Scenario: Emit pageMargins and pageSetup

- **WHEN** `ws.pageSetup = { orientation: "landscape", paperSize: 9, margins: { top: 1, bottom: 1, left: 1, right: 1, header: 0.5, footer: 0.5 } }`
- **THEN** the sheet XML contains `<pageMargins top="1" .../>` and `<pageSetup orientation="landscape" paperSize="9"/>`

#### Scenario: printArea becomes a defined name

- **WHEN** `ws.pageSetup = { printArea: "A1:D10" }`
- **THEN** the workbook defines `_xlnm.Print_Area` scoped to the sheet with value `Sheet1!$A$1:$D$10`

### Requirement: Reader parses page setup and print area

The reader SHALL parse `<pageMargins>` and `<pageSetup>` from the sheet XML
into `ws.pageSetup`, and resolve `_xlnm.Print_Area` / `_xlnm.Print_Titles`
defined names back into `ws.pageSetup.printArea` / `printTitles`.

#### Scenario: Read page setup from Excel/ExcelJS

- **WHEN** a sheet XML contains `<pageSetup orientation="portrait" paperSize="1"/>` and `<pageMargins top="0.75" .../>`
- **THEN** `ws.pageSetup.orientation === "portrait"`, `paperSize === 1`, and margins are populated

