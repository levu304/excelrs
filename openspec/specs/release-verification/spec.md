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

The release pipeline SHALL verify the streaming reader and writer round-trip **before** any
package is published, over a multi-sheet workbook whose cell payload is materially larger than
a spot check, asserting that every streamed row and cell value matches between write and read.
The writer's memory model — an input phase that buffers sheets and an output phase that streams
them — is stated by `openspec/specs/streaming-xlsx/spec.md` and is not restated here.

The assertion SHALL run in the `build` job against every build-matrix target's own binary, and
SHALL fail the release if it does not hold. It SHALL cover every cell value shape the streaming
API accepts — number, text, boolean, and formula — so that scaling the assertion up does not
narrow the set of value types it exercises.

#### Scenario: Streaming round-trip on a large workbook

- **WHEN** the release smoke test streams a large workbook through the streaming writer, then
  reads it back through the streaming reader
- **THEN** the read-back row count and cell values equal what was streamed, and the release job
  SHALL fail if they do not

#### Scenario: Streaming round-trip fails before any package is published

- **WHEN** the streaming round-trip assertion fails for a matrix target
- **THEN** the release SHALL fail without publishing the platform or main package

#### Scenario: Streaming round-trip is exercised at a materially larger payload

- **WHEN** the streaming smoke test runs
- **THEN** the workbook it writes SHALL carry a cell payload above a stated floor spanning
  multiple sheets, so that truncation, corruption, and archive-format limits at scale are
  detected rather than passed
- **AND** the payload the floor is measured against SHALL count only the bytes the workbook
  actually emits, so the floor cannot be satisfied by text that is never written

#### Scenario: Every streamed value round-trips at scale

- **WHEN** the streaming smoke test reads back the workbook it wrote
- **THEN** every row count and every cell value it wrote SHALL match what was read, not merely
  the first row or a sampled subset

#### Scenario: Every cell value shape round-trips

- **WHEN** the streaming smoke test writes a workbook
- **THEN** it SHALL write at least one cell of each value shape the streaming API accepts —
  number, text, boolean, and formula — and SHALL assert each of them survives the round-trip

#### Scenario: Streaming path does not regress in-memory path

- **WHEN** the release smoke test runs both the in-memory and streaming round-trips
- **THEN** both SHALL pass, and the streaming assertions SHALL be added alongside the in-memory
  ones rather than replacing them

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

### Requirement: Release assertions complete before publication

Every assertion the release pipeline makes about how a built artifact BEHAVES SHALL run before
the first `npm publish` step in the `publish` job. The release SHALL NOT publish any package
before those assertions have passed for that package's target, and the publish steps SHALL be
ordered after the assertion steps that gate them.

This requirement governs behavioral assertions only. Assertions about PACKAGING — that a
published artifact resolves from the registry, that its entrypoint and binary are present and
loadable, that its optional dependency set is correct — cannot be made before publication,
because the subject of the assertion does not exist on npm until the publish step has run. Such
packaging assertions SHALL be scoped to resolution and loadability and SHALL NOT re-assert
behavioral guarantees already gated before publication.

#### Scenario: A failing behavioral assertion blocks publication

- **WHEN** a behavioral assertion about a built artifact fails
- **THEN** no package SHALL have been published to npm by that run, and the failure SHALL be
  attributable to the job step that made it

#### Scenario: Assertion steps precede publish steps in the job definition

- **WHEN** the `publish` job's steps are read in order
- **THEN** every step asserting release behavior SHALL appear before the first step that
  publishes a package to npm

#### Scenario: Post-publish steps assert packaging, not behavior

- **WHEN** a step in the `publish` job runs after the first `npm publish` step
- **THEN** it SHALL assert only that the published package set resolves and loads, and SHALL NOT
  re-assert a behavioral guarantee that a pre-publish step already gates

### Requirement: Release verification claims name the step that implements them

A requirement in this capability that attributes a verification activity to the release
pipeline SHALL name the workflow step that performs it, such that a reader can locate the
assertion without inferring it from the job's purpose. Where an activity is performed per
build-matrix target, the requirement SHALL name the per-target mechanism rather than a
single post-publish step. Where the activity runs on only some build-matrix targets, the
requirement SHALL name every target class it covers, so that adding coverage to a previously
uncovered target does not leave the requirement understating what ships.

#### Scenario: The streaming assertion names a step that runs it

- **WHEN** a reader looks for the streaming round-trip assertion
- **THEN** the requirement SHALL name the workflow step and job that perform it, and that step
  SHALL be the one carrying the streaming assertions

#### Scenario: A widened target set updates the requirement

- **WHEN** the streaming round-trip assertion gains coverage for a build-matrix target class it
  did not previously cover
- **THEN** the requirement naming it SHALL be updated in the same change to name that class
