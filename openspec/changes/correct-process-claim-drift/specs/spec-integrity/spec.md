# Spec Delta

## ADDED Requirements

### Requirement: An agreement claim names the check that verifies it

A requirement that asserts a specification agrees with a machine-readable source — a workflow
file, a package manifest, a build configuration — SHALL name the check that compares the two,
or SHALL be scoped to a statement that cannot become untrue. Naming the source is not
sufficient: an invariant with no comparing check reads as verified while drifting, and no
validator can distinguish a satisfied agreement from an unenforced one.

#### Scenario: A verified agreement names its check

- **WHEN** a requirement asserts that a specification and a machine-readable source agree
- **THEN** it SHALL name the check that compares them, and that check SHALL read both

#### Scenario: An unchecked agreement is not stated as a standing invariant

- **WHEN** a requirement asserts an agreement and no check reads both artifacts
- **THEN** the requirement SHALL be scoped historically or retired, rather than left as a
  standing claim nothing verifies

### Requirement: An agreement claim does not resolve to an artifact that cannot support it

Where a requirement asserts an agreement, the comparison SHALL be over the properties the
agreement is actually about. A check that verifies only a resolvable target — a path that
exists, a status that reads `current` — SHALL NOT be treated as verifying properties of that
target the check never inspects, since resolution and relevance are independent and a check can
pass while the property it appears to cover is absent.

#### Scenario: A resolvable target is not evidence for an uninspected property

- **WHEN** a citation resolves to an existing file whose status satisfies a stated rule, but the
  cited decision is not the one that file records
- **THEN** the citation SHALL be treated as incorrect, not as compliant

#### Scenario: A check verifies the property it names

- **WHEN** a check is cited as the mechanism guaranteeing an agreement
- **THEN** it SHALL inspect the specific properties the agreement asserts, and a change that
  removes one of those properties from the source SHALL cause the check to fail