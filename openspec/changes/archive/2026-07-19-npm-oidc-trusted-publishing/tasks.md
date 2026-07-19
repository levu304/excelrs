## 1. Register trusted publishers on npmjs.com (manual, off-repo)

- [x] 1.1 For `@levu304/excelrs`: add trusted publisher — owner `levu304`,
  repository `excelrs`, workflow `release.yml`, allowed action `npm publish`.
- [x] 1.2 Repeat for `@levu304/excelrs-darwin-arm64`,
  `@levu304/excelrs-linux-x64-gnu`, `@levu304/excelrs-win32-x64-msvc` (same
  workflow file, each package authorizes itself).
- [x] 1.3 For **all four** packages, set the package publish-access requirement
  to **“Require two-factor authentication and disallow tokens”** (strictest
  setting). OIDC publish is unaffected (it is not a long-lived token); this
  removes the token-publish attack surface. Do **not** use the
  “2FA or GAT with bypass” option, which npm is deprecating (Jan 2027).

## 2. Strip token auth from `release.yml`

- [x] 2.1 Remove `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}` from the `publish`
  job `env:` block.
- [x] 2.2 Remove the "Authenticate to npm" step (the `echo "…_authToken=…" >
  .npmrc` + `npm whoami` lines). `npm publish` will auto-detect OIDC.
- [x] 2.3 Bump `node-version: 20` → `node-version: 22.14.0` in the publish job's
  `actions/setup-node` step (and the build matrix's setup-node, for
  toolchain parity).

## 3. Bump Node in `ci.yml`

- [x] 3.1 Change `node-version: 20` → `node-version: 22.14.0` in `ci.yml`'s
  `actions/setup-node` step.

## 4. Verify and retire the token

- [x] 4.1 Push the workflow changes and cut a test `v*` tag (or run
  `workflow_dispatch`) — confirm all four packages publish via OIDC and the
  verify + smoke-test gate passes.
- [x] 4.2 Once an OIDC publish is confirmed green, delete the `NPM_TOKEN`
  secret from the repository's GitHub settings.
- [x] 4.3 Confirm the change is archived and `release-verification` spec reflects the OIDC requirement.
