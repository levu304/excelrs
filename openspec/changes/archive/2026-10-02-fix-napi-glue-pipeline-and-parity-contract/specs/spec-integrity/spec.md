# Spec Delta

## ADDED Requirements

### Requirement: A requirement naming an enforcement mechanism names a resolvable target

A requirement that names a build step, script, or check as the thing that guarantees its
behavior SHALL name a target that step actually operates on — a path the build emits, or a
file the check loads. A requirement SHALL NOT assert that a mechanism maintains a guarantee
when that mechanism is wired to a different artifact than the one the guarantee is about,
since such a requirement reads as satisfied while the mechanism is inert.

#### Scenario: A hook aimed at a file the build does not emit is caught

- **WHEN** a requirement states that a build-time hook maintains a guarantee, and the hook's
  target filename is not a file the build produces
- **THEN** the requirement SHALL be treated as failed, not as satisfied

#### Scenario: A check pointed at the wrong artifact does not certify the build

- **WHEN** a release check loads a file that the build does not produce, in order to verify
  the build output
- **THEN** that check SHALL NOT be treated as evidence about the build output

### Requirement: An unenforceable contract is not stated as a standing invariant

A requirement SHALL NOT mandate the ongoing accuracy of a value that no validation, test, or
build step can check. Where a project keeps such a value for readers, the requirement
SHALL either name a checkable source of truth for it or be scoped to a historical statement
that cannot become untrue.

#### Scenario: A hand-maintained table is not a standing invariant

- **WHEN** a requirement would mandate that a hand-maintained document stay accurate and no
  automated check reads that document
- **THEN** the requirement SHALL be retired or restated against a checkable source, rather
  than left to decay

#### Scenario: A historical record needs no ongoing enforcement

- **WHEN** a requirement records what was true at a past release
- **THEN** it MAY stand without an enforcement mechanism, because the past does not change
