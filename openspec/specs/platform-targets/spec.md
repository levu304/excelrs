# platform-targets Specification

## Purpose

Guarantees that every platform the release builds for is actually deliverable to a consumer
on that platform, stated once as an invariant so the target list can grow without the
specification becoming untrue.

## Requirements

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

Each target in the release build matrix SHALL be verified by loading that target's own binary
and asserting a minimal round-trip through the native addon, in the `build` job, before the
release publishes any package. The release SHALL fail if that target's verification fails. The
requirement does not depend on a single consolidated post-publish check: the per-target
mechanism is the one that gates publication.

#### Scenario: A target's smoke test failure blocks the release

- **WHEN** the functional smoke test fails for any build-matrix target
- **THEN** the release SHALL fail before publishing

#### Scenario: The check is per-target rather than consolidated after publication

- **WHEN** a reader asks what gates publication for a given matrix target
- **THEN** the gating verification SHALL be identified as that target's own build-job
  verification, not as a check that runs once after the packages are already on npm

### Requirement: The build matrix and declared targets agree

The set of targets in the `release.yml` build matrix and the set of targets declared in
`package.json` `napi.targets` SHALL be identical, so a locally built addon covers every
platform the release ships.

#### Scenario: A target added to the matrix is also declared for local builds

- **WHEN** a target is added to the release build matrix
- **THEN** it SHALL also appear in `package.json` `napi.targets` without a specification
  change

### Requirement: musl binaries are dynamically linked and load only on musl hosts

A musl-targeted binary SHALL link its musl libc dynamically and SHALL NOT ship as a fully
static musl cdylib. A fully static musl cdylib segfaults when `dlopen`ed, because its static
TLS and pthread initialization collide with the host libc, so dynamic linkage is a
correctness requirement rather than a packaging preference. Because the binary records its
musl libc as a dynamic dependency, it loads on a musl host of the same architecture and fails
to load — with a loader error, not a crash — on a host whose libc is not musl. Selecting it on
a non-musl host is therefore a consumer-side routing decision made by the loader from the
host libc and architecture, not a property the binary provides.

#### Scenario: A musl consumer loads the addon on a musl host

- **WHEN** the musl binary is loaded in a Node.js process whose host libc is musl and whose
  architecture matches the binary
- **THEN** the addon SHALL load and round-trip a workbook successfully

#### Scenario: A non-musl host attempts to load the musl binary

- **WHEN** the musl binary is loaded in a Node.js process whose host libc is not musl
- **THEN** the load SHALL fail with a loader error naming the unresolved musl libc
  dependency, and SHALL NOT segfault or load successfully

#### Scenario: A musl binary is built against a fully static musl libc

- **WHEN** a musl-targeted binary is built such that it records no dynamic musl libc
  dependency
- **THEN** the release SHALL reject it before publishing
