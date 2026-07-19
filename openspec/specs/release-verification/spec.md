# release-verification Specification

## Purpose
TBD - created by archiving change fix-issue-3-release-hardening. Update Purpose after archive.
## Requirements
### Requirement: Release smoke test verifies styled round-trip

The release pipeline SHALL round-trip a styled `.xlsx` workbook through both the write and read paths and assert that cell-level style survives the read, so a read-path style-loss regression fails the release before publish.

#### Scenario: Styled workbook round-trips through the read path

- **WHEN** the release smoke test writes a workbook with a cell styled `font.bold = true` and `fill.foreground = "FFFF0000"`, then reads that workbook back from bytes
- **THEN** the read-back cell SHALL report `font.bold = true` and `fill.foreground = "FFFF0000"`, and the release job SHALL fail if either assertion is false

#### Scenario: Existing writer-only behavior is preserved

- **WHEN** the release smoke test runs the existing writer exercise (`setCellStyle` → `write()`)
- **THEN** it SHALL continue to pass, and the new read-path assertions SHALL be added on top of it rather than replacing it

### Requirement: Release smoke test round-trips merges and row styles

The `release.yml` functional smoke test SHALL, in addition to the cell `font.bold` + `fill.foreground` round-trip, write a workbook containing a merged range and a row-level style, read it back, and assert both survive; the release job SHALL fail if either assertion is false.

#### Scenario: Merged range and row style survive the release smoke test

- **WHEN** the release smoke test writes a workbook with a merged range and a styled row, then reads it back from bytes
- **THEN** the read-back worksheet SHALL report the merged range and the row style, and the release job SHALL fail if either is missing

### Requirement: Release smoke test exercises the streaming round-trip

The `release.yml` functional smoke test SHALL, in addition to the in-memory
styled round-trip, drive the streaming reader and writer over a large workbook
(one whose row count exceeds practical in-memory bounds) and assert the
streamed rows and cell values match between write and read, so a streaming
regression fails the release before publish.

#### Scenario: Streaming round-trip on a large workbook

- **WHEN** the release smoke test streams a large workbook through the streaming writer, then reads it back through the streaming reader
- **THEN** the read-back row count and cell values equal what was streamed, and the release job SHALL fail if they do not

#### Scenario: Streaming path does not regress in-memory path

- **WHEN** the release smoke test runs both the in-memory and streaming round-trips
- **THEN** both SHALL pass, and the streaming assertions SHALL be added alongside the in-memory ones rather than replacing them

### Requirement: Release publishes via npm trusted publishing (OIDC)

The `release.yml` publish job SHALL authenticate to npm via trusted publishing
(OIDC) rather than a long-lived token. No write credential SHALL be stored in
repository secrets or written to a `.npmrc` during release. Each of the four
published packages (`@levu304/excelrs`, `@levu304/excelrs-darwin-arm64`,
`@levu304/excelrs-linux-x64-gnu`, `@levu304/excelrs-win32-x64-msvc`) SHALL have
a trusted-publisher configuration on npmjs.com authorizing the `release.yml`
workflow to perform `npm publish`.

#### Scenario: Publish succeeds without NPM_TOKEN

- **WHEN** a `v*` tag triggers `release.yml` and no `NPM_TOKEN` secret is
  present in the environment
- **THEN** the four `npm publish` calls SHALL succeed via OIDC token exchange,
  and the publish job SHALL fail if OIDC is not configured

#### Scenario: No long-lived credential persists

- **WHEN** the release pipeline runs
- **THEN** no `_authToken` SHALL be written to any `.npmrc` file, and the
  repository SHALL hold no npm write token in its secrets store

