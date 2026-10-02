# Spec Delta

## Purpose

Guarantees that every platform the release builds for is actually deliverable to a consumer
on that platform, stated once as an invariant so the target list can grow without the
specification becoming untrue.

## ADDED Requirements

### Requirement: Every build-matrix target produces a named native artifact

Each target in the `release.yml` build matrix SHALL produce a native addon artifact named
`excelrs.<npm_dir>.node` and upload it under npm directory `<npm_dir>`, where `npm_dir` is
the matrix entry's declared value.

#### Scenario: A matrix entry builds and uploads its artifact

- **WHEN** the release build job runs for a matrix entry with `npm_dir: <npm_dir>`
- **THEN** it SHALL produce `excelrs.<npm_dir>.node` and upload it as the `<npm_dir>` artifact

### Requirement: Every build-matrix target is resolvable at install time

Each target in the release build matrix SHALL be published as a platform package and
declared by the main package such that a consumer on a matching host resolves it as the
native binary without a source build.

#### Scenario: A consumer on a matrix platform resolves the prebuilt binary

- **WHEN** a consumer whose host matches a build-matrix target installs the package
- **THEN** the platform package SHALL resolve as the native binary and the package SHALL load
  without a source compile

### Requirement: Every build-matrix target runs the functional smoke test

Each target in the release build matrix SHALL be verified by running the functional smoke
test against that target's binary before the release is published, and the release SHALL fail
if that target's smoke test fails.

#### Scenario: A target's smoke test failure blocks the release

- **WHEN** the functional smoke test fails for any build-matrix target
- **THEN** the release SHALL fail before publishing

### Requirement: The build matrix and declared targets agree

The set of targets in the `release.yml` build matrix and the set of targets declared in
`package.json` `napi.targets` SHALL be identical, so a locally built addon covers every
platform the release ships.

#### Scenario: A target added to the matrix is also declared for local builds

- **WHEN** a target is added to the release build matrix
- **THEN** it SHALL also appear in `package.json` `napi.targets` without a specification
  change

### Requirement: musl binaries load without a host-matched libc

A musl-targeted binary SHALL link its musl libc dynamically so it loads in any Node.js
process regardless of the host's libc, and a fully statically linked musl binary SHALL NOT
be shipped.

#### Scenario: A musl consumer loads the addon on a non-musl-glibc host

- **WHEN** the musl binary is loaded in a Node.js process whose host libc is not musl
- **THEN** the addon SHALL load and round-trip a workbook successfully
