## ADDED Requirements

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
