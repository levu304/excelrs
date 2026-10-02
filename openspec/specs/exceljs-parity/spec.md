# exceljs-parity Specification

## Purpose

Records the outcome of the v1.x drop-in [ExcelJS](https://github.com/exceljs/exceljs) 4.4.0
feature-parity program, which was declared complete at release v2.0.0. Introduced by change
`v0-10-0-exceljs-roadmap-align`; reduced to a historical record by change
`fix-napi-glue-pipeline-and-parity-contract`, which retired the requirements mandating the
ongoing accuracy of the `ROADMAP.md` parity matrix — a hand-maintained table that no
validation reads, so those obligations decayed without detection.

For what a feature area does today, read that area's capability under `openspec/specs/*`. For
what shipped in which release, read `CHANGELOG.md`.

## Requirements

### Requirement: The ExcelJS-4.4.0 v1.x parity program is recorded complete at v2.0.0

The v1.x drop-in ExcelJS-4.4.0 parity program is **complete** as of release v2.0.0: every
feature area in the v1.x targeted roadmap (including `streaming XLSX`, and all v0.x-v1.x
areas) was marked `shipped` (or `partial` where explicitly noted), and the ROADMAP SHALL
record the program as complete. This requirement is a historical record.

#### Scenario: v2.0.0 records the program complete

- **WHEN** the v2.0.0 parity record is read
- **THEN** every feature area in the v1.x targeted roadmap SHALL be recorded as `shipped`, or `partial` where explicitly noted

#### Scenario: A later release does not amend this record

- **WHEN** a release after v2.0.0 ships a feature area this record describes
- **THEN** the area's current behavior SHALL be read from that area's capability under `openspec/specs/*`, and this historical record SHALL NOT require amendment

### Requirement: v2.0.0 parity program exclusions

The v2.0.0 parity completion record SHALL list these areas as excluded from the completed
v1.x program: charts, pivot tables, and formula evaluation (distant / deferred), plus
themes-write, sheet state (visible/hidden), tab color, and default worksheet properties
(deferred to post-v2.0.0 triage).

#### Scenario: Excluded areas were distant or deferred at v2.0.0

- **WHEN** the v2.0.0 parity record is read
- **THEN** charts, pivot tables, and formula evaluation SHALL be recorded as distant or deferred

#### Scenario: Post-v2.0.0 deferrals are named

- **WHEN** the v2.0.0 parity record is read
- **THEN** themes-write, sheet state, tab color, and default worksheet properties SHALL be recorded as deferred to post-v2.0.0 triage
