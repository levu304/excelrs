# Tasks

Group 1 lands before group 2 on purpose: the checker goes in first and is expected to
*fail* against the current workflows, which is the evidence that it detects this class of
drift. Group 2 then makes it pass.

The probe suite copies the real repo into a sandbox per case, so until group 2 runs, every
sandboxed tree carries the real drift and the `expectAccepted` cases fail on it. Tasks 1.2
and 1.3 are therefore verified after group 2 completes, not at the point the checker is
added. 1.1 is verified immediately, since a red run against the unmodified repo is the
evidence being sought.

## 1. Extend the parity checker and its probe suite

- [x] 1.1 Add a type-declaration flag comparison to `scripts/verify-feature-parity.cjs`, reusing the existing command-flattening step and the existing `package.json`-anchored reference invocation rather than a second parser or a hardcoded flag list. Verify by running `node scripts/verify-feature-parity.cjs` against the unmodified repo: it SHALL exit 1 and name all three workflow invocations as missing the flags, since the drift is still present at this point.

- [x] 1.2 Add probe cases to `__test__/verify-feature-parity.test.ts` using the existing `expectRejected` / `expectAccepted` / `replace` helpers: dropping a flag from a `ci.yml` invocation is rejected naming that invocation; dropping one from the `release.yml` musl step is rejected; a `release.yml` invocation that carries `--target` and `--cross-compile` but the full flag set is accepted; a workflow declaring a flag the `package.json` scripts do not is rejected. Verify with `npx vitest run __test__/verify-feature-parity.test.ts`.

- [x] 1.3 Replace the two verbatim step copies in the probe suite with a structural lookup by `- name:` label, so adding a flag to a build step cannot silently void a probe. Verify three ways: the suite is green; re-introducing the fixed-6-space `STEP_RE` bug fails exactly the re-indent probe; making invocation discovery chunk-level fails exactly the second-build probe. No probe case was removed and no assertion weakened.

## 2. Correct the workflow build invocations

- [x] 2.1 Add the two type-declaration flags to the build step in `.github/workflows/ci.yml`. Verify by re-running `node scripts/verify-feature-parity.cjs` and confirming `ci.yml` is no longer named in the output.

- [x] 2.2 Add the same two flags to the musl cross-compile step in `.github/workflows/release.yml`. Verify by re-running the checker and confirming the musl step is no longer named.

- [x] 2.3 Add the same two flags to the non-musl emnapi step in `.github/workflows/release.yml`. Verify with `pnpm verify:features`: it SHALL exit 0 and report all five `napi build` invocations agreeing.

## 3. Integration verification

- [x] 3.1 Run `pnpm build` then `pnpm verify:build` — guards against the flag addition changing what the `--pipe` transform produces. Verify exit code 0 and that the OK line still reports the pipe transforms and the entrypoint glue. Note: this branch is cut from `main`, whose `verify-build-output.cjs` predates the declaration-form comparison, so the OK line here reads `pipe transforms present, entrypoint glue intact`. The stronger half of this check — that published and generated declarations agree — can only be observed on the dependent PR once this change has landed on `main`.

- [x] 3.2 Confirm the diff touches no published surface: the changed-file list against `main` is only the two workflow files, the checker, and its probe suite, plus the OpenSpec change directory — no change to `index.d.ts`, `index.js`, `package.json`, or `src/`. Verify by reading the stat output.

- [ ] 3.3 Push and confirm the dependent PR's gate goes green with `gh pr checks`. All three legs SHALL pass. Note in the PR body that the release matrix legs (musl, emnapi) cannot be exercised locally and are the residual unverified surface, per design.md Risks.