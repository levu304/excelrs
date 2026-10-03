# Spec Delta

## ADDED Requirements

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