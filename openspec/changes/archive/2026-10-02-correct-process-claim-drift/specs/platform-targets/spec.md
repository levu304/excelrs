# Spec Delta

## REMOVED Requirements

### Requirement: musl binaries load without a host-matched libc

## MODIFIED Requirements

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

## ADDED Requirements

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