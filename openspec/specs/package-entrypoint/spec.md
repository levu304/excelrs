# package-entrypoint Specification

## Purpose

Guarantees that the ExcelJS-compat JavaScript surface consumers depend on — the `getCell`
method overloads and the type declarations that describe the cell-value unions — is delivered
by a step the pipeline actually runs, rather than asserted by a requirement that describes a
mechanism no build step invokes.

## Requirements

### Requirement: Worksheet.getCell resolves by A1 address

The package SHALL expose a `getCell` overload on `Worksheet` that accepts an A1-style address
and returns the cell at that address.

#### Scenario: Worksheet.getCell resolves by A1 address

- **WHEN** a consumer calls `worksheet.getCell("A1")`
- **THEN** it returns the cell addressed by `A1`

### Requirement: Worksheet.getCell resolves by row and column

The package SHALL expose a `getCell` overload on `Worksheet` that accepts a row and a column
and returns the cell at that position.

#### Scenario: Worksheet.getCell resolves by row and column

- **WHEN** a consumer calls `worksheet.getCell(2, 3)`
- **THEN** it returns the cell at row 2, column 3

### Requirement: Row.getCell resolves by column number or letter

The package SHALL expose a `getCell` method on `Row` that accepts either a column number or a
column letter and returns the cell in that column.

#### Scenario: Row.getCell resolves by column number

- **WHEN** a consumer calls `row.getCell(5)`
- **THEN** it returns the cell in column 5 of that row

#### Scenario: Row.getCell resolves by column letter

- **WHEN** a consumer calls `row.getCell("E")`
- **THEN** it returns the cell in column E of that row

### Requirement: The getCell overloads are declared in the published types

The package's published type declarations SHALL declare every `getCell` overload form the
runtime supports, so a TypeScript consumer can call each form without a cast or a compile
error.

#### Scenario: Each overload form type-checks

- **WHEN** a TypeScript consumer calls `worksheet.getCell` with an address or with a row and
  column, or `row.getCell` with a column number or a column letter
- **THEN** each call SHALL type-check against the published declarations

### Requirement: The overloads survive a source change to the entrypoint

The `getCell` overloads SHALL be present on the entrypoint the package's `main` field names,
and that entrypoint SHALL be maintained as a hand-authored source file rather than a build
output. Regenerating the build's disposable output SHALL NOT remove the overloads, and
regenerating the entrypoint by hand SHALL re-establish them.

#### Scenario: Rebuilding the native addon preserves the overloads

- **WHEN** the native addon is rebuilt
- **THEN** the entrypoint named by `main` SHALL still expose `getCell` on both `Worksheet` and
  `Row`

#### Scenario: The build does not overwrite the entrypoint

- **WHEN** the build emits its JavaScript and type-declaration outputs
- **THEN** it SHALL NOT write to the entrypoint named by `main`

### Requirement: The pipeline runs the step that produces the generated type transforms

Every build the pipeline performs — continuous integration and release — SHALL run the same
post-build transformation of generated type declarations that a local build runs. The
transformed declarations are a build output consumed by type-checking; they are not part of
the published package, whose type declarations are hand-maintained and unaffected by this
transform.

#### Scenario: CI and release apply the same transform as a local build

- **WHEN** the pipeline builds the native addon for continuous integration or for release
- **THEN** it SHALL invoke the same post-build type-declaration transformation that the
  project's local build script invokes

#### Scenario: A build that skips the transform is not equivalent

- **WHEN** a build omits the post-build type-declaration transformation
- **THEN** its generated type declarations SHALL be treated as not equivalent to a local
  build's, and a check that reads the generated declarations SHALL fail

#### Scenario: The transform does not reach a consumer

- **WHEN** the package is packed for publication
- **THEN** the transformed generated declarations SHALL NOT be among the published files,
  and the published type declarations SHALL be unchanged by the transform

### Requirement: The generated type declarations are verified in CI

Continuous integration SHALL assert, against the type declarations the build actually produces,
that the cell-value union types and the refined cell-value setter are present. A check that
reads only hand-maintained files SHALL NOT satisfy this requirement.

#### Scenario: A missing type transform fails CI

- **WHEN** the build produces type declarations lacking the cell-value union or the refined
  setter type
- **THEN** the CI job SHALL fail

#### Scenario: The assertion reads the build output

- **WHEN** the CI check evaluates the cell-value union
- **THEN** it SHALL read the generated type-declaration file, not a hand-maintained mirror of
  it
