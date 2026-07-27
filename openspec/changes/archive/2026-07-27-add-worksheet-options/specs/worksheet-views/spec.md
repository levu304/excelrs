## ADDED Requirements

### Requirement: SheetView carries showGridLines

A `SheetView` SHALL expose a `showGridLines` optional boolean field. When
`true`, the sheet SHALL display grid lines in the viewing application. The
default when absent in OOXML is `true`.

#### Scenario: Set showGridLines

- **WHEN** `ws.views = [{ showGridLines: false }]`
- **THEN** `ws.views[0].showGridLines === false`

#### Scenario: showGridLines defaults to true when absent

- **WHEN** a sheet view is set without specifying `showGridLines`
- **THEN** the property SHALL be `undefined` / `None` (omitted from serialization,
  leaving the OOXML default of `true`)

### Requirement: Writer emits showGridLines attribute

When `showGridLines` is set on a `SheetView`, the writer SHALL emit a
`showGridLines` attribute on the `<sheetView>` element. The attribute value
SHALL be `"0"` for `false` and `"1"` for `true`. When `showGridLines` is
`None` (unset), the attribute SHALL be omitted.

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

### Requirement: Reader parses showGridLines

The reader SHALL parse the `showGridLines` attribute from `<sheetView>` in
`xl/worksheets/sheetN.xml`. When the attribute is absent, `showGridLines`
SHALL be `None`. When present as `"0"` or `"1"`, the boolean value SHALL be
set accordingly.

#### Scenario: Read showGridLines="0"

- **WHEN** the sheet XML contains `<sheetView showGridLines="0">`
- **THEN** `ws.views[0].showGridLines === false`

#### Scenario: Read showGridLines="1"

- **WHEN** the sheet XML contains `<sheetView showGridLines="1">`
- **THEN** `ws.views[0].showGridLines === true`

#### Scenario: Missing showGridLines is None

- **WHEN** the sheet XML contains `<sheetView>` without `showGridLines`
- **THEN** `ws.views[0].showGridLines` is `undefined` / `None`
