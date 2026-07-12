# data-validation Specification

## Purpose
TBD - created by archiving change v0-8-0. Update Purpose after archive.
## Requirements
### Requirement: Worksheet exposes a data-validation API

A `Worksheet` SHALL expose data-validation CRUD keyed by `sqref` range: a
`dataValidations` getter returning all validations,
`addDataValidation(dv: DataValidation)` (upsert by `dv.sqref`, ignoring
duplicate ranges), `getDataValidation(sqref: String)` returning
`Option<DataValidation>`, and `removeDataValidation(sqref: String)` (no-op
if absent). All clones of a Worksheet SHALL share the same validation state
via `Arc<Mutex<>>`.

#### Scenario: Add and read back a validation

- **WHEN** `ws.addDataValidation({ sqref: "A1:A10", type: "whole", operator: "between", formula1: "1", formula2: "10" })`
- **THEN** `ws.getDataValidation("A1:A10")` returns a `DataValidation` with `type === "whole"`, `operator === "between"`, `formula1 === "1"`, `formula2 === "10"`, `sqref === "A1:A10"`

#### Scenario: Upsert by sqref

- **WHEN** `addDataValidation({ sqref: "A1:A10", ...dv1 })` then `addDataValidation({ sqref: "A1:A10", ...dv2 })`
- **THEN** `ws.dataValidations` has exactly one entry for `"A1:A10"` (dv2 wins); `removeDataValidation("A1:A10")` leaves zero entries

#### Scenario: Remove absent range is no-op

- **WHEN** `removeDataValidation("NonExistent")`
- **THEN** no error is thrown; `ws.dataValidations` length is unchanged

### Requirement: Writer emits data validations per sheet

The writer SHALL emit a `<dataValidations count="{n}">` element in each
sheet XML (after `<hyperlinks>`) with `<dataValidation>` sub-elements
carrying `type`, optional `operator`, `sqref`, and `<formula1>`/`<formula2>`
children. Boolean attributes SHALL be `="1"` only when true, omitted
otherwise.

#### Scenario: Emit a whole/between validation

- **WHEN** a worksheet has a `whole`/`between` validation on `A1:A10` with `allowBlank: true`
- **THEN** the sheet XML contains `<dataValidations count="1">` and `<dataValidation type="whole" operator="between" allowBlank="1" sqref="A1:A10"><formula1>1</formula1><formula2>10</formula2></dataValidation>`

#### Scenario: Empty worksheet omits dataValidations

- **WHEN** a worksheet has no data validations added
- **THEN** the sheet XML SHALL NOT contain a `<dataValidations>` element

### Requirement: Reader parses data validations from sheet XML

The reader SHALL parse `<dataValidations>` from `xl/worksheets/sheetN.xml`
directly from the zip archive (calamine exposes none) and attach them to the
corresponding `Worksheet` by sheet index.

#### Scenario: Read a validation written by Excel or exceljs

- **WHEN** an `.xlsx` has `<dataValidation type="whole" allowBlank="1" sqref="A1"><formula1>1</formula1><formula2>10</formula2></dataValidation>`
- **THEN** `ws.getDataValidation("A1")` returns `type === "whole"`, `formula1 === "1"`, `formula2 === "10"`

#### Scenario: File without dataValidations

- **WHEN** a workbook has no `<dataValidations>` in any sheet
- **THEN** `ws.dataValidations` is empty for every sheet; reader does not error

