# Spec Delta

## MODIFIED Requirements

### Requirement: Every build-matrix target runs the functional smoke test

Each target in the release build matrix SHALL be verified by loading that target's own binary
and asserting a minimal round-trip through the native addon, in the `build` job, before the
release publishes any package. The release SHALL fail if that target's verification fails. The
requirement does not depend on a single consolidated post-publish check: the per-target
mechanism is the one that gates publication.

#### Scenario: A target's smoke test failure blocks the release

- **WHEN** the functional smoke test fails for any build-matrix target
- **THEN** the release SHALL fail before publishing

#### Scenario: The check is per-target rather than consolidated after publication

- **WHEN** a reader asks what gates publication for a given matrix target
- **THEN** the gating verification SHALL be identified as that target's own build-job
  verification, not as a check that runs once after the packages are already on npm