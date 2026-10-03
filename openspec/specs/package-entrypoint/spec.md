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

### Requirement: Exported enums are declared in a form transpiling consumers can import

The published type declarations SHALL declare every enum the package exports in a form a
TypeScript consumer can import when the consumer's compiler options enable isolated
modules. A declaration that requires compile-time substitution across module boundaries is
not such a form.

#### Scenario: A consumer with isolated modules imports an exported enum

- **WHEN** a TypeScript consumer whose compiler options enable isolated modules imports
  any enum the package exports
- **THEN** the import SHALL compile without error and the imported member SHALL be usable
  as a value

#### Scenario: An imported enum member resolves at runtime

- **WHEN** a consumer references a member of an exported enum as a runtime value
- **THEN** the package's entrypoint SHALL provide that member's value, so the reference
  does not evaluate to undefined

### Requirement: Published declarations cover every name the build generates

The published type declarations SHALL declare every name the generated type declarations
declare. The generated declarations are produced by the build from the native surface, so
a name they declare and the published declarations omit is a name a consumer cannot import.

#### Scenario: A name only the build generates is caught

- **WHEN** the generated type declarations declare a name the published declarations omit
- **THEN** the check that compares them SHALL fail, naming the missing name

#### Scenario: An additional published alias is permitted

- **WHEN** the published declarations declare a name the generated declarations do not,
  and that name is an alias of a declared type rather than a distinct shape
- **THEN** the comparison SHALL NOT fail on account of that name

#### Scenario: The comparison reads the build output

- **WHEN** the coverage check evaluates which names are declared
- **THEN** it SHALL read the generated type-declaration file the build produces, not a
  hand-maintained mirror of it

### Requirement: Enum declaration form agrees between published and generated declarations

The published type declarations and the generated type declarations SHALL agree on whether
each enum they both declare is declared const. A published declaration that diverges from
the form the build emits misdescribes the enum to every consumer of the package.

#### Scenario: A const divergence is caught

- **WHEN** an enum is declared const in the published declarations and not const in the
  generated declarations
- **THEN** the check that compares them SHALL fail, naming the enum

#### Scenario: Agreement is required in both directions of change

- **WHEN** a future build is configured to emit a declaration form the published
  declarations do not match
- **THEN** the check SHALL fail rather than leaving the divergence to reach a release

### Requirement: Build invocations agree on type-declaration flags

Every `napi build` invocation in `package.json` and in the GitHub workflows SHALL
declare the same type-declaration flags, anchored on the build scripts in
`package.json`. An invocation that omits a flag the `package.json` scripts declare
emits declarations in a different form, so a check that reads only the local build
cannot satisfy this requirement.

#### Scenario: A workflow invocation missing a declared flag is caught

- **WHEN** a `napi build` invocation in a workflow omits a type-declaration flag that
  the `package.json` build scripts declare
- **THEN** the parity check SHALL fail, naming the invocation and the flag

#### Scenario: The expected flag set follows package.json

- **WHEN** the type-declaration flags declared by the `package.json` build scripts
  change
- **THEN** the comparison SHALL use the new set without a specification change

#### Scenario: The comparison reads the invocations themselves

- **WHEN** the parity check evaluates which flags an invocation declares
- **THEN** it SHALL read the `napi build` invocations in `package.json` and the
  workflows, not a hand-maintained mirror of them
