## MODIFIED Requirements

### Requirement: Release publishes via npm trusted publishing (OIDC)

The `release.yml` publish job SHALL authenticate to npm via trusted publishing
(OIDC) rather than a long-lived token. No write credential SHALL be stored in
repository secrets or written to a `.npmrc` during release. Each of the **five**
published packages (`@levu304/excelrs`, `@levu304/excelrs-darwin-arm64`,
`@levu304/excelrs-linux-x64-gnu`, `@levu304/excelrs-linux-arm64-gnu`,
`@levu304/excelrs-win32-x64-msvc`) SHALL have a trusted-publisher configuration
on npmjs.com authorizing the `release.yml` workflow to perform `npm publish`.

#### Scenario: Publish succeeds without NPM_TOKEN

- **WHEN** a `v*` tag triggers `release.yml` and no `NPM_TOKEN` secret is
  present in the environment
- **THEN** the five `npm publish` calls SHALL succeed via OIDC token exchange,
  and the publish job SHALL fail if OIDC is not configured

#### Scenario: No long-lived credential persists

- **WHEN** the release pipeline runs
- **THEN** no `_authToken` SHALL be written to any `.npmrc` file, and the
  repository SHALL hold no npm write token in its secrets store

### Requirement: Patch release SHALL follow existing release process

Patch releases SHALL follow the existing release-verification requirements.
No new requirements beyond what the release-verification spec already defines.

#### Scenario: Patch release uses same CI pipeline

- **WHEN** a `v2.2.1` patch tag is pushed
- **THEN** the existing Release workflow SHALL build, test, verify, and publish
  all **5** npm packages without workflow modifications
