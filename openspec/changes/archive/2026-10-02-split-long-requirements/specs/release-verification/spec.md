## REMOVED Requirements

### Requirement: Release publishes via npm trusted publishing (OIDC)

**Reason**: Combined the OIDC authentication model, the seven-package enumeration, and the spec/workflow parity rule into one over-long requirement.

**Migration**: Replaced by "Release publishes via npm trusted publishing (OIDC)", "Trusted publishing is configured for the seven published packages", and "Release package set matches the release-verification spec".

## ADDED Requirements

### Requirement: Release publishes via npm trusted publishing rather than a stored token

The `release.yml` publish job SHALL authenticate to npm via trusted publishing (OIDC)
rather than a long-lived token. No write credential SHALL be stored in repository secrets
or written to a `.npmrc` during release.

#### Scenario: Release publishes without a stored npm token

- **WHEN** the publish job runs a release
- **THEN** it SHALL authenticate via trusted publishing and no write credential SHALL be present in repository secrets or a generated `.npmrc`

### Requirement: Trusted publishing is configured for the seven published packages

Each of the seven published packages SHALL have a trusted-publisher configuration on
npmjs.com authorizing the `release.yml` workflow to perform `npm publish`:
`@levu304/excelrs`, `@levu304/excelrs-darwin-arm64`,
`@levu304/excelrs-linux-x64-gnu`, `@levu304/excelrs-linux-arm64-gnu`,
`@levu304/excelrs-linux-x64-musl`, `@levu304/excelrs-linux-arm64-musl`, and
`@levu304/excelrs-win32-x64-msvc`.

#### Scenario: Every published package can be trusted-published

- **WHEN** each of the seven packages is published by the release workflow
- **THEN** each SHALL have an npm trusted-publisher configuration authorizing the `release.yml` workflow

#### Scenario: Both musl platform packages are covered

- **WHEN** the trusted-publisher configurations are reviewed
- **THEN** `@levu304/excelrs-linux-x64-musl` and `@levu304/excelrs-linux-arm64-musl` SHALL both be present

### Requirement: Release package set matches the release-verification spec

The package count and names listed in this capability SHALL match the set the publish job
publishes and verifies.

#### Scenario: Spec and workflow agree on the package set

- **WHEN** the package names in this capability are compared with those the publish job publishes
- **THEN** the two sets SHALL be identical in both count and names