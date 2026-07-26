## ADDED Requirements

### Requirement: Trigger release-please

CI SHALL run the release-please workflow after merge to main to create a new npm version.

#### Scenario: Release PR created

- **WHEN** PR #36 is merged into main
- **THEN** release-please shall create a release PR with version bump to v2.3.0
- **THEN** the release PR shall include updated CHANGELOG and version

### Requirement: Publish to npm

The system SHALL publish the new version to npm after the release PR is merged.

#### Scenario: npm publish succeeds

- **WHEN** the release PR is merged
- **THEN** the release tag shall be created
- **THEN** the npm package shall be published

### Requirement: Release verification

CI SHALL verify the release tag and npm publish completed successfully.

#### Scenario: Release tag verified

- **WHEN** the release is complete
- **THEN** a git tag `v2.3.0` shall exist on main
- **THEN** the npm registry shall show the new version
