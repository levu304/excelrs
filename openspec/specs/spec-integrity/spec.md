# spec-integrity Specification

## Purpose

Keeps the excelrs OpenSpec corpus mechanically valid and discoverable, so the specs under
`openspec/specs/` can be trusted as the description of what the project actually ships
rather than drifting silently between releases.

## Requirements

### Requirement: Spec corpus is validated in CI

The CI pipeline SHALL run OpenSpec validation over the project's specs and in-flight
changes on every push and pull request targeting the main branch, and SHALL fail the
build on any ERROR-level finding. Validation SHALL be non-interactive so its result does
not depend on a terminal.

#### Scenario: A structurally invalid spec fails the build

- **WHEN** a spec under `openspec/specs/` is missing a required section such as `## Requirements`
- **THEN** the CI job SHALL fail with a non-zero exit status, naming the invalid spec

#### Scenario: A valid corpus passes

- **WHEN** every spec and in-flight change passes validation
- **THEN** the CI step SHALL exit zero and the build SHALL proceed

#### Scenario: Validation is non-interactive

- **WHEN** the validation step runs in CI
- **THEN** it SHALL run with interactive prompting disabled and SHALL never block waiting for input

### Requirement: Every capability is discoverable through the standard inventory

Every capability under `openspec/specs/` SHALL use the canonical main-spec section
structure (`## Purpose` followed by `## Requirements`) so that it is reported by the
standard spec inventory with its full requirement count. A spec SHALL NOT use change-delta
section headers (`## ADDED Requirements`, `## REMOVED Requirements`, `## MODIFIED
Requirements`) as its main-spec structure.

#### Scenario: Delta-archived spec reports its requirements

- **WHEN** a spec's main-spec file uses `## Requirements` rather than delta headers
- **THEN** the spec inventory SHALL report that capability with a non-zero requirement count equal to its `### Requirement` blocks

#### Scenario: Structural repair preserves requirement text

- **WHEN** a delta-archived spec is converted to the canonical main-spec structure
- **THEN** every requirement name and requirement text SHALL be preserved verbatim, with no requirement added, removed, or reworded

### Requirement: Specs carry authored Purpose sections

Every capability under `openspec/specs/` SHALL have a `## Purpose` section describing what
that capability is for, in prose written for the project. A capability SHALL NOT retain the
archive-time placeholder text (`TBD - created by archiving change ...`) as its Purpose.

#### Scenario: Placeholder purpose is reported as a finding

- **WHEN** a capability's `## Purpose` still reads `TBD - created by archiving change ...`
- **THEN** strict validation SHALL report a WARNING-level finding for that capability

#### Scenario: Purpose describes the capability

- **WHEN** a capability's `## Purpose` is read
- **THEN** it SHALL state what the capability is for in the project's own words, without a `TBD` or `TODO` marker

### Requirement: Requirements state invariants, not restated inventories

A requirement SHALL state a property that holds as the implementation evolves. It SHALL NOT
restate a list, count, or set that is already recorded in a machine-readable source such as a
build configuration, manifest, or workflow. Where a requirement must refer to such a set, it
SHALL name the source and require agreement with it rather than enumerate its contents.

#### Scenario: A set grows without making a requirement untrue

- **WHEN** an entry is added to a set that a requirement refers to
- **THEN** the requirement SHALL remain satisfied without amendment

#### Scenario: The named source and the specification disagree

- **WHEN** a set recorded in the named machine-readable source and the set a requirement
  describes diverge
- **THEN** the requirement SHALL be treated as failed, not as satisfied

### Requirement: A requirement states one behavior

Each requirement SHALL state a single behavior, and a requirement covering several behaviors
SHALL be split into separate requirements with their own scenarios. Enumerated fields,
attribute maps, and threshold lists SHALL appear in scenarios rather than in the normative
description.

#### Scenario: A requirement's scope can be validated in isolation

- **WHEN** a requirement is evaluated
- **THEN** a reader can determine whether it holds without consulting another requirement

### Requirement: A requirement does not cite itself as its own subject

A requirement SHALL NOT derive its truth from the specification that contains it. Where a
specification and an external artifact must agree, the requirement SHALL name the external
artifact as the source of truth.

#### Scenario: Agreement is checked against a real artifact

- **WHEN** a requirement asserts that the specification matches something
- **THEN** it SHALL name the concrete artifact being compared against, outside the
  specification
