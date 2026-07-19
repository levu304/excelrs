## Why

npm v12 (now `latest`) begins deprecating the most sensitive uses of 2FA-bypass
Granular Access Tokens (GATs). A GAT configured to bypass 2FA will lose the
ability to perform sensitive account/package actions (token create/delete,
package access, maintainer management) starting **early August 2026**, and will
**stop publishing directly around January 2027**. `excelrs` currently
authenticates its publish step with `secrets.NPM_TOKEN` — a long-lived token
written into a local `.npmrc` at release time. If that token is a 2FA-bypass
GAT, the publish pipeline breaks on the January 2027 deadline; even before
then, a long-lived write token is a standing exposure (leak = write access to
all four published packages).

Trusted publishing (OIDC) eliminates the long-lived token entirely: each
publish exchanges a short-lived, per-job OIDC ID token for a registry token
that cannot be extracted or replayed. The publish job already declares
`permissions: id-token: write`, so half the setup is done.

## What Changes

- **Remove long-lived token from the release pipeline.** Delete the
  `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}` env binding and the
  "Authenticate to npm" step (which echoes `_authToken` into a `.npmrc` and runs
  `npm whoami`) from `release.yml`. `npm publish` then falls back to OIDC
  automatic detection.
- **Register trusted publishers on npmjs.com for all four packages.**
  `@levu304/excelrs`, `@levu304/excelrs-darwin-arm64`,
  `@levu304/excelrs-linux-x64-gnu`, `@levu304/excelrs-win32-x64-msvc` each get a
  trusted-publisher entry: owner `levu304`, repository `excelrs`, workflow
  `release.yml`, allowed action `npm publish`. (Manual step — done once on the
  npm website, not in this repo.)
- **Bump Node.js to 22.14.0 in both workflows.** `release.yml` and `ci.yml`
  currently pin `node-version: 20`, which ships npm ~10.x (no OIDC support;
  Node 20 also reached EOL April 2026). Node 22.14.0 ships npm 11.x, which
  satisfies the OIDC requirement (`npm CLI ≥ 11.5.1` / `Node ≥ 22.14.0`) and is
  LTS through April 2027.
- **Delete the `NPM_TOKEN` secret** from the repository's GitHub settings once
  the first OIDC publish succeeds.

## Capabilities

### New Capabilities

<!-- none: this is a CI/publishing-auth change with no library API or behavior change -->

### Modified Capabilities

- `release-verification`: the publish step SHALL authenticate via npm trusted
  publishing (OIDC) rather than a long-lived token, so no write credential is
  stored in repo secrets or written to disk during release.

## Impact

- **Code:** `.github/workflows/release.yml` (drop auth step + env, bump
  `node-version`) and `.github/workflows/ci.yml` (bump `node-version` only). No
  Rust, no `index.d.ts`, no `package.json` changes.
- **Manual (off-repo):** four trusted-publisher registrations on npmjs.com;
  deletion of the `NPM_TOKEN` repo secret after the first successful OIDC
  publish.
- **Out of scope:** staged publishing (alternative not adopted — direct OIDC
  publish is sufficient for a single-workflow, four-package native addon);
  npm provenance attestation (can be layered on later once OIDC is in place);
  `allowScripts`/`--allow-git`/`--allow-remote` defaults — already a non-issue
  (no lifecycle scripts, no git/URL deps).
