## ADDED Requirements

### Requirement: Release publishes via npm trusted publishing (OIDC)

The `release.yml` publish job SHALL authenticate to npm via trusted publishing
(OIDC) rather than a long-lived token. No write credential SHALL be stored in
repository secrets or written to a `.npmrc` during release. Each of the four
published packages (`@levu304/excelrs`, `@levu304/excelrs-darwin-arm64`,
`@levu304/excelrs-linux-x64-gnu`, `@levu304/excelrs-win32-x64-msvc`) SHALL have
a trusted-publisher configuration on npmjs.com authorizing the `release.yml`
workflow to perform `npm publish`.

#### Scenario: Publish succeeds without NPM_TOKEN

- **WHEN** a `v*` tag triggers `release.yml` and no `NPM_TOKEN` secret is
  present in the environment
- **THEN** the four `npm publish` calls SHALL succeed via OIDC token exchange,
  and the publish job SHALL fail if OIDC is not configured

#### Scenario: No long-lived credential persists

- **WHEN** the release pipeline runs
- **THEN** no `_authToken` SHALL be written to any `.npmrc` file, and the
  repository SHALL hold no npm write token in its secrets store
