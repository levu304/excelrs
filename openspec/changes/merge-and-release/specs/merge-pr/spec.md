## ADDED Requirements

### Requirement: Merge PR checkout and merge

CI SHALL merge PR #36 into main using rebase strategy.

#### Scenario: Successful merge

- **WHEN** PR #36 is open and CI passes
- **THEN** the branch shall be merged into main with rebase

#### Scenario: Merge blocked by CI

- **WHEN** PR #36 checks are not passing
- **THEN** merge shall be blocked until CI passes

### Requirement: Post-merge branch cleanup

CI SHALL delete the merged branch after successful merge.

#### Scenario: Branch deleted after merge

- **WHEN** PR #36 is merged into main
- **THEN** the remote branch `fix/typed-enum-declarations` shall be deleted
