## ADDED Requirements

### Requirement: addWorksheet accepts optional AddWorksheetOptions

A `Workbook` SHALL expose `addWorksheet(name, options?)` where `options` is an
optional `AddWorksheetOptions` object. When passed, the options SHALL be applied
to the newly created `Worksheet` before it is returned. When omitted, behavior
SHALL be identical to the existing single-argument signature.

The `AddWorksheetOptions` object SHALL support the following optional fields:

| Field | Type | Maps to |
| ------- | ------ | --------- |
| `pageSetup` | `PageSetup` | `ws.pageSetup` setter |
| `views` | `Array<Partial<SheetView>>` | `ws.views` setter |
| `headerFooter` | `HeaderFooter` | `ws.headerFooter` setter |
| `protection` | `SheetProtection` | `ws.protection` setter |
| `autoFilter` | `string` | `ws.autoFilter` setter |

#### Scenario: Create worksheet with pageSetup

- **WHEN** `workbook.addWorksheet("Sheet1", { pageSetup: { orientation: "landscape", paperSize: 9 } })`
- **THEN** the returned worksheet has `pageSetup.orientation === "landscape"` and `pageSetup.paperSize === 9`

#### Scenario: Create worksheet with views

- **WHEN** `workbook.addWorksheet("Sheet1", { views: [{ state: "frozen", xSplit: 1, ySplit: 2 }] })`
- **THEN** the returned worksheet has `views[0].state === "frozen"`, `xSplit === 1`, `ySplit === 2`

#### Scenario: Create worksheet with all options

- **WHEN** `workbook.addWorksheet("Sheet1", { pageSetup: { orientation: "portrait" }, views: [{ showGridLines: false }], headerFooter: { oddHeader: "&CHeader" }, protection: { locked: true } })`
- **THEN** all fields are set on the returned worksheet matching the input

#### Scenario: Single-arg call unchanged

- **WHEN** `workbook.addWorksheet("Sheet1")`
- **THEN** a blank worksheet is returned with no page setup, no views, no header/footer — identical to current behavior

### Requirement: Options applied atomically before return

The options SHALL be applied to the worksheet before it is pushed into the
workbook's internal worksheet list and before the cloned reference is returned
to the caller. A worksheet created with options SHALL have the same id and name
as a worksheet created without options.

#### Scenario: Id assignment unaffected by options

- **WHEN** `workbook.addWorksheet("Sheet1", { pageSetup: { orientation: "landscape" } })` is the first call
- **THEN** the returned worksheet has `id === 1`

### Requirement: AddWorksheetOptions maps to a no-copy napi-object struct

The `AddWorksheetOptions` type SHALL be defined as a `#[napi(object)]` struct in
Rust and as an `export interface` in TypeScript. Each field SHALL be `Option<T>`
on the Rust side to match the optional JS shape. Excluding a field SHALL leave
the corresponding worksheet property at its default (unset).

#### Scenario: Partial options leaves other fields at defaults

- **WHEN** `workbook.addWorksheet("Sheet1", { autoFilter: "A1:C10" })` is called
- **THEN** `ws.autoFilter === "A1:C10"`, `ws.pageSetup` is `null`, `ws.views` is `[]`
