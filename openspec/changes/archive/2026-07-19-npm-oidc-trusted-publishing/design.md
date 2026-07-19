## Context

`release.yml` publishes `@levu304/excelrs` plus three platform packages
(`-darwin-arm64`, `-linux-x64-gnu`, `-win32-x64-msvc`) from a single workflow
triggered on `v*` tags. The `publish` job currently authenticates with
`secrets.NPM_TOKEN` written into a repo-root `.npmrc`. The four `npm publish`
calls (three platform packages, then the main package with injected
`optionalDependencies`) run under that token.

npm v12 (July 2026) deprecates 2FA-bypass GATs: account operations in August
2026, direct publishing in January 2027. Trusted publishing (OIDC) replaces the
long-lived token with a per-job short-lived credential minted from GitHub's
OIDC token (`ACTIONS_ID_TOKEN_REQUEST_TOKEN`, available because the job already
declares `permissions: id-token: write`).

The publish job also pins `node-version: 20`; npm OIDC requires `npm CLI ≥
11.5.1` or `Node ≥ 22.14.0`, so the Node version must rise. `ci.yml` pins the
same Node 20 and should rise in lockstep to avoid divergent toolchains.

## Goals / Non-Goals

**Goals:**

- Publish all four packages with no long-lived write token in repo secrets.
- Keep the existing multi-package publish order (platform packages first, then
  main package) and the post-publish verify + smoke-test gate unchanged.
- Get the workflow onto a supported Node LTS (22.x) so OIDC works natively and
  Node 20 EOL (April 2026) is behind us.

**Non-Goals:**

- Not adopting **staged publishing** — it adds a manual promotion step with no
  benefit for a single-workflow native addon whose publish already self-verifies
  via the smoke test.
- Not adding **npm provenance** attestation in this change — OIDC is the
  prerequisite; provenance can be layered on afterward (it needs the same
  `id-token: write` + a `--provenance` flag).
- Not changing the verify step, smoke tests, or any Rust code.

## Decisions

- **Direct OIDC publish, not staged.** The existing `npm view` + functional
  smoke test already gates correctness after publish; staged publishing's
  inspect-then-promote model is redundant here.
- **One trusted-publisher entry per package, all pointing at `release.yml`.**
  Each package independently authorizes the workflow to publish *it*. The OIDC
  token is scoped to the workflow + ref, so a `npm publish` for
  `@levu304/excelrs-darwin-arm64` is authorized only because that package's
  trusted-publisher config lists `release.yml`. No extra auth orchestration is
  needed across the four publishes — every `npm publish` call just works.
- **Bump `node-version: 22.14.0` in both `release.yml` and `ci.yml`.** Picks up
  npm 11.x (OIDC-capable) and an LTS through April 2027. Avoids `npm install -g
  npm@12` in-step churn. `corepack enable` + `pnpm install --frozen-lockfile`
  already insulate dependency install from npm's own version, so CI is
  unaffected by the bump.

- **Set each package's publish-access requirement to “Require 2FA and disallow
  tokens” (not “2FA or GAT with bypass”).** This is the strictest npm package
  setting and pairs naturally with OIDC: OIDC trusted publishing authenticates
  via a short-lived OIDC exchange, which is *not* a long-lived token, so the
  “disallow tokens” clause does not block CI publishes. With OIDC in place, no
  long-lived token is needed for automated publishing, so retaining token-based
  publish access (the “2FA or GAT with bypass” option) only preserves an attack
  surface — a leaked PAT/GAT could still publish. Disallowing tokens closes that
  surface entirely; emergency manual publishes from a maintainer machine still
  work via interactive 2FA. The “GAT with bypass 2FA” path is also being
  deprecated by npm itself (Jan 2027), so choosing “disallow tokens”
  future-proofs the setting rather than depending on a mechanism npm is
  retiring.

## Risks / Trade-offs

- [Risk] The four trusted-publisher entries on npmjs.com are **manual** and
  live off-repo; a missing entry fails only that package's publish at release
  time. → Mitigation: the change's first cut should be a dry run on a test tag
  (or rely on the existing verify loop to surface a failed publish), and the
  deletion of `NPM_TOKEN` is deferred until after a confirmed OIDC publish.
- [Risk] `npm publish` requires `npm CLI ≥ 11.5.1`. If `actions/setup-node@…`
  with `node-version: 22.14.0` resolves an npm below that, OIDC detection
  silently falls back to token auth and fails (no token present). →
  Mitigation: pin the setup-node major to one that ships npm 11+ (Node 22.14.0
  does); verify with `npm --version` in the workflow if desired.
- [Risk] Removing `NPM_TOKEN` before the trusted publishers are live bricks the
  pipeline. → Mitigation: delete the secret **last**, only after a successful
  OIDC publish is observed.
