# release-verification Specification

## Purpose
Defines what the release pipeline must prove before publishing: smoke tests that round-trip styled, merged, row-styled, and streamed workbooks through the native addon, plus the npm trusted-publishing (OIDC) requirements that keep write credentials out of the repository.
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

### Requirement: Release publishes via npm trusted publishing rather than a stored token

The `release.yml` publish job SHALL authenticate to npm via trusted publishing (OIDC)
rather than a long-lived token. No write credential SHALL be stored in repository secrets
or written to a `.npmrc` during release.

#### Scenario: Release publishes without a stored npm token

- **WHEN** the publish job runs a release
- **THEN** it SHALL authenticate via trusted publishing and no write credential SHALL be present in repository secrets or a generated `.npmrc`

### Requirement: Trusted publishing is configured for every published package

Every package published by the release workflow SHALL have a trusted-publisher configuration
on npmjs.com authorizing the `release.yml` workflow to perform `npm publish`. The set of
packages this applies to SHALL be the set the publish job publishes, as declared by
`release.yml`.

#### Scenario: Every published package can be trusted-published

- **WHEN** the release workflow publishes its package set
- **THEN** every package in that set SHALL have a trusted-publisher configuration
  authorizing the `release.yml` workflow

#### Scenario: A newly added platform package is covered

- **WHEN** a platform package is added to the publish job's package set
- **THEN** it SHALL have a trusted-publisher configuration without a specification change
