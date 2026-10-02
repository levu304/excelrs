# Spec Delta

## MODIFIED Requirements

### Requirement: Release smoke test exercises the streaming round-trip

The release pipeline SHALL verify the streaming reader and writer round-trip **before** any
package is published, over a multi-sheet workbook whose cell payload is materially larger than
a spot check, asserting that every streamed row and cell value matches between write and read.
The writer's memory model — an input phase that buffers sheets and an output phase that streams
them — is stated by `openspec/specs/streaming-xlsx/spec.md` and is not restated here. The
assertion SHALL run in the `build` job, against each non-musl matrix target's own binary, and
SHALL fail the release if it does not hold.

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

#### Scenario: Every streamed value round-trips at scale

- **WHEN** the streaming smoke test reads back the workbook it wrote
- **THEN** every row count and every cell value it wrote SHALL match what was read, not merely
  the first row or a sampled subset

#### Scenario: Streaming path does not regress in-memory path

- **WHEN** the release smoke test runs both the in-memory and streaming round-trips
- **THEN** both SHALL pass, and the streaming assertions SHALL be added alongside the in-memory
  ones rather than replacing them

## ADDED Requirements

### Requirement: Release assertions complete before publication

Every behavioral assertion the release pipeline makes about a built artifact SHALL run before
the first `npm publish` step in the `publish` job. The release SHALL NOT publish any package
before those assertions have passed for that package's target, and the publish steps SHALL be
ordered after the assertion steps that gate them.

#### Scenario: A failing assertion blocks publication

- **WHEN** a release assertion fails
- **THEN** no package SHALL have been published to npm by that run, and the failure SHALL be
  attributable to the job step that made it

#### Scenario: Assertion steps precede publish steps in the job definition

- **WHEN** the `publish` job's steps are read in order
- **THEN** every step asserting release behavior SHALL appear before the first step that
  publishes a package to npm

### Requirement: Release verification claims name the step that implements them

A requirement in this capability that attributes a verification activity to the release
pipeline SHALL name the workflow step that performs it, such that a reader can locate the
assertion without inferring it from the job's purpose. Where an activity is performed per
build-matrix target, the requirement SHALL name the per-target mechanism rather than a
single post-publish step.

#### Scenario: The streaming assertion names a step that runs it

- **WHEN** a reader looks for the streaming round-trip assertion
- **THEN** the requirement SHALL name the workflow step and job that perform it, and that step
  SHALL be the one carrying the streaming assertions