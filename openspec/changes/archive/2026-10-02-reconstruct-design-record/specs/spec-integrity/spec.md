# Spec Delta

## ADDED Requirements

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
