## NEW Requirements

### Requirement: Parse defined names from xl/workbook.xml

The workbook reader SHALL parse the `<definedNames>` element in `xl/workbook.xml`, extracting each `<definedName>` entry's `name` attribute, text content (the OOXML formula/range string), and optional `localSheetId` attribute. Sheet-scoped names (with `localSheetId`) SHALL be resolved to the corresponding sheet name using the sheet ordering from `<sheets>` (0-based index). Workbook-scoped names (without `localSheetId`) SHALL have `sheet = None`.

#### Scenario: Global defined name

- **WHEN** `xl/workbook.xml` contains `<definedName name="SalesTotal">Sheet1!$A$1</definedName>`
- **THEN** the parsed `DefinedName` has `name = "SalesTotal"`, `value = "Sheet1!$A$1"`, `sheet = None`

#### Scenario: Sheet-scoped defined name

- **WHEN** `<definedName name="LocalRef" localSheetId="0">$A$1:$B$10</definedName>` and sheet at index 0 is named "Sheet1"
- **THEN** the parsed name has `name = "LocalRef"`, `value = "$A$1:$B$10"`, `sheet = Some("Sheet1")`

#### Scenario: Multiple defined names

- **WHEN** workbook.xml contains two `<definedName>` entries
- **THEN** both are returned in parse order; no entries are merged or deduped

#### Scenario: No defined names

- **WHEN** `xl/workbook.xml` has no `<definedNames>` element
- **THEN** the function returns an empty `Vec`

#### Scenario: localSheetId out of range

- **WHEN** `<definedName localSheetId="99"` exceeds the workbook's sheet count
- **THEN** the entry is treated as workbook-scoped (`sheet = None`), preserving the data without hard error

### Requirement: Write defined names to xl/workbook.xml

The workbook writer SHALL emit a `<definedNames>` child of `<workbook>` (after `<sheets>)` when the workbook has any defined names. Each `<definedName>` element SHALL carry the `name` attribute, optional `localSheetId` (0-based index) for sheet-scoped names, and the raw value text as element content. When no defined names exist, the `<definedNames>` element SHALL be omitted entirely.

#### Scenario: Write global defined name

- **WHEN** workbook has `DefinedName { name: "TaxRate", value: "0.08", sheet: None }`
- **THEN** the writer emits `<definedName name="TaxRate">0.08</definedName>`

#### Scenario: Write sheet-scoped defined name

- **WHEN** workbook has `DefinedName { name: "Total", value: "Sheet2!$B$5", sheet: Some("Sheet2") }` and "Sheet2" is at index 1 in the worksheets vec
- **THEN** the writer emits `<definedName name="Total" localSheetId="1">Sheet2!$B$5</definedName>`

#### Scenario: Empty defined names list omits element

- **WHEN** `defined_names` is empty
- **THEN** no `<definedNames>` element is emitted in `xl/workbook.xml`

#### Scenario: Sheet name not found emits global name

- **WHEN** name references `sheet = Some("MissingSheet")` but no worksheet with that name exists
- **THEN** the writer emits a workbook-scoped `<definedName>` (no `localSheetId`)

### Requirement: Workbook API for defined names

The `Workbook` napi-rs class SHALL expose a getter returning a snapshot of all defined names, and methods to add, remove, and retrieve individual names.

#### Scenario: definedNames getter

- **WHEN** `wb.addDefinedName("Rate", "0.08")` then `wb.definedNames`
- **THEN** returns `[{ name: "Rate", value: "0.08", sheet: null }]`

#### Scenario: addDefinedName with sheet scope

- **WHEN** `wb.addDefinedName("Local", "Sheet1!$A$1", "Sheet1")`
- **THEN** `wb.definedNames` includes entry with `sheet = "Sheet1"`

#### Scenario: addDefinedName upsert

- **WHEN** `wb.addDefinedName("X", "1")` then `wb.addDefinedName("X", "2")`
- **THEN** only one entry with name="X" exists, value="2" (workspace-scope upsert; sheet-scope upsert matches by name+sheet)

#### Scenario: removeDefinedName

- **WHEN** `wb.addDefinedName("X", "1")` then `wb.removeDefinedName("X")`
- **THEN** `wb.definedNames` is empty

#### Scenario: removeDefinedName absent name is no-op

- **WHEN** `wb.removeDefinedName("NonExistent")`
- **THEN** no error, definedNames unchanged

#### Scenario: getDefinedName returns single entry

- **WHEN** `wb.getDefinedName("Rate")` after add
- **THEN** returns the `DefinedName` object; returns `null` for missing name
