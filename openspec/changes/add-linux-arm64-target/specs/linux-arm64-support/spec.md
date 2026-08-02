## Purpose

The excelrs package SHALL provide a prebuilt native addon for aarch64 Linux so
that consumers on arm64 Linux hosts (Apple Silicon in arm64 containers, AWS
Graviton, Raspberry Pi 4/5, arm64 CI runners) can load the package without
forcing x86_64 emulation or falling back to a source build.

## ADDED Requirements

### Requirement: Release build matrix includes the aarch64-unknown-linux-gnu target

The `release.yml` build matrix SHALL include an entry targeting
`aarch64-unknown-linux-gnu` that produces an `excelrs.linux-arm64-gnu.node`
binary. The entry's `npm_dir` SHALL be `linux-arm64-gnu`.

#### Scenario: New matrix entry builds a native arm64 binary

- **WHEN** the release build job runs with `target: aarch64-unknown-linux-gnu`
- **THEN** the job SHALL produce `excelrs.linux-arm64-gnu.node` and upload it as
  the `linux-arm64-gnu` build artifact

### Requirement: linux-arm64 binary is published as an optional dependency

The published main package SHALL declare
`@levu304/excelrs-linux-arm64-gnu` in `optionalDependencies`, and the
`napi.targets` array in `package.json` SHALL include `aarch64-unknown-linux-gnu`,
so npm resolves the matching binary automatically on arm64 Linux.

#### Scenario: arm64 Linux consumer installs the arm64 binary

- **WHEN** a consumer on `aarch64-unknown-linux-gnu` runs
  `npm install @levu304/excelrs@<version>`
- **THEN** npm SHALL resolve `@levu304/excelrs-linux-arm64-gnu` as the native
  binary and `require('@levu304/excelrs')` SHALL succeed without a source build

### Requirement: Release job runs the arm64 smoke test

The `release.yml` publish job SHALL run the functional smoke test against the
arm64 Linux binary as part of release verification: it SHALL load the package,
build+write a styled workbook, read it back, and assert style + merge +
row-style survival, failing the release if any assertion is false.

#### Scenario: Styled workbook round-trips on arm64 Linux

- **WHEN** the release smoke test on the arm64 build writes a workbook with a
  cell styled `font.bold = true` and `fill.foreground = "FFFF0000"`, then reads
  it back from bytes
- **THEN** the read-back cell SHALL report `font.bold = true` and
  `fill.foreground = "FFFF0000"`, and the release job SHALL fail if either
  assertion is false

#### Scenario: arm64 smoke test does not regress in-memory or streaming behavior

- **WHEN** the arm64 smoke test runs the in-memory and streaming round-trips
- **THEN** both SHALL pass, and the release job SHALL fail if either fails
