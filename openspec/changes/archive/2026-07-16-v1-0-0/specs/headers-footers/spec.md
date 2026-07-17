## ADDED Requirements

### Requirement: Worksheet exposes header and footer

A `Worksheet` SHALL expose a `headerFooter` getter returning the current
header/footer descriptor (with `oddHeader`, `oddFooter`, `evenHeader`,
`evenFooter`, `firstHeader`, `firstFooter`, `alignWithMargins`,
`differentFirst`, `differentOddEven`) and a setter accepting such a descriptor.
All `&`-prefixed format codes SHALL be stored verbatim (no parsing).

#### Scenario: Set an odd header/footer

- **WHEN** `ws.headerFooter = { oddHeader: "&LSheet&CRight", oddFooter: "&P of &N" }`
- **THEN** `ws.headerFooter.oddHeader === "&LSheet&CRight"` and `ws.headerFooter.oddFooter === "&P of &N"`

#### Scenario: Format codes pass through verbatim

- **WHEN** a header string contains codes like `&[Page]` / `&D` / `&T`
- **THEN** the string is stored and emitted unchanged (no validation or rewriting)

### Requirement: Writer emits headerFooter element

When a worksheet has a `headerFooter`, the writer SHALL emit `<headerFooter>`
(with only the present odd/even/first children) at the CT_Worksheet position
(after sheet views, before `pageMargins`/`pageSetup`). A worksheet without a
`headerFooter` SHALL NOT emit `<headerFooter>`.

#### Scenario: Emit headerFooter

- **WHEN** `ws.headerFooter = { oddHeader: "Title", oddFooter: "Foot" }`
- **THEN** the sheet XML contains `<headerFooter><oddHeader>Title</oddHeader><oddFooter>Foot</oddFooter></headerFooter>`

#### Scenario: No headerFooter omits element

- **WHEN** a worksheet has no `headerFooter` set
- **THEN** the sheet XML SHALL NOT contain a `<headerFooter>` element

### Requirement: Reader parses headerFooter

The reader SHALL parse `<headerFooter>` and its odd/even/first children from
`xl/worksheets/sheetN.xml` and populate `ws.headerFooter`, preserving format
codes. A sheet without `<headerFooter>` SHALL leave `ws.headerFooter` `null`.

#### Scenario: Read headerFooter from Excel/ExcelJS

- **WHEN** a sheet XML contains `<headerFooter><oddHeader>&CSum</oddHeader></headerFooter>`
- **THEN** `ws.headerFooter.oddHeader === "&CSum"`

#### Scenario: File without headerFooter

- **WHEN** a workbook has no `<headerFooter>` in any sheet
- **THEN** `ws.headerFooter` is `null` for every sheet; reader does not error
