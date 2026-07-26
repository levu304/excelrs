# release-verification — Delta

This delta records the execution of the existing release-verification
specification for the v2.2.1 patch release. No requirement changes.

## ADDED Requirements

### Requirement: Patch release SHALL follow existing release process

Patch releases SHALL follow the existing release-verification requirements.
No new requirements beyond what the release-verification spec already defines.

#### Scenario: Patch release uses same CI pipeline

- **WHEN** a `v2.2.1` patch tag is pushed
- **THEN** the existing Release workflow SHALL build, test, verify, and publish
  all 4 npm packages without workflow modifications
