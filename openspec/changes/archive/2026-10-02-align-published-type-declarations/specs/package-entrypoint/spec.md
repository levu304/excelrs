# Spec Delta

## ADDED Requirements

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
