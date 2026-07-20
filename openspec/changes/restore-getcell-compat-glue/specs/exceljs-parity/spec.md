## ADDED Requirements

### Requirement: excelrs preserves ExcelJS-compat getCell overloads

`excelrs` SHALL expose ExcelJS-compatible `getCell` overloads on `Worksheet` and `Row` that delegate to the native `getCellBy*` Rust APIs. The overloads SHALL survive `napi build` — they SHALL be re-injected through a build-time hook, never hand-patched into the generated `index.js` / `index.d.ts`.

#### Scenario: Worksheet.getCell resolves by A1 address

- **WHEN** a consumer calls `worksheet.getCell("A1")`
- **THEN** it returns the cell via the native `getCellByAddress` API

#### Scenario: Worksheet.getCell resolves by row and column

- **WHEN** a consumer calls `worksheet.getCell(2, 3)`
- **THEN** it returns the cell via the native `getCellByRc` API

#### Scenario: Row.getCell resolves by column number

- **WHEN** a consumer calls `row.getCell(5)`
- **THEN** it returns the cell via the native `getCellByColNum` API

#### Scenario: Row.getCell resolves by column letter

- **WHEN** a consumer calls `row.getCell("E")`
- **THEN** it returns the cell via the native `getCellByColLetter` API

#### Scenario: Glue survives napi build

- **WHEN** `napi build` regenerates `index.js` / `index.d.ts`
- **THEN** the `getCell` overloads are re-injected automatically and no manual re-patch is required
