# Tasks

## 1. Feature-set parity check

- [x] 1.1 Create `scripts/verify-feature-parity.cjs` that extracts `--features` values from every script in `package.json` and from every `napi build` invocation in `.github/workflows/*.yml`, normalizes them into sets, and exits non-zero naming the file and differing features when the sets disagree. Verify by running it against the current tree and confirming it exits zero, then temporarily removing `--features formula-eval` from `package.json`'s `build` script and confirming it exits non-zero naming both files.
- [x] 1.2 Add a `verify:features` script to `package.json` pointing at the new checker, and verify `pnpm verify:features` exits zero on the unmodified tree.
- [x] 1.3 Add a CI step running `node scripts/verify-feature-parity.cjs` immediately before the existing `verify-build-output` step, and verify by running the CI workflow's build-and-verify steps locally with the flag temporarily removed from one build script — the CI step must fail.

## 2. Pre-publish release gate

- [x] 2.1 Add a pre-publish assertion script that loads the downloaded `linux-x64-gnu` binary through the hand-maintained `index.js` glue and asserts the same three guarantees the current Functional smoke test asserts: a styled cell round-trips `font.bold` and `fill.foreground`, a merged range survives, and a row style survives. Verify by running it against a locally built addon and confirming all three assertions pass, then corrupting the read path in a scratch build and confirming it fails.
- [x] 2.2 Insert that assertion as a new step in the `publish` job positioned **after** `Download platform artifacts` (line 229) and **before** `Publish platform packages` (line 300). Verify by reading the job's steps in order and confirming no assertion step follows the first publish step.
- [x] 2.3 Rename the existing post-publish step from `Functional smoke test` to a name stating it verifies the published package set resolves and loads from the registry, and update its inline comments so the two steps no longer read as duplicates. Verify by grepping `.github/workflows/release.yml` for `Functional smoke test` and confirming zero matches, and confirming the renamed step still runs after both publish steps.
- [ ] 2.4 Confirm the pre-publish assertion fails the release before any package is published by running the `publish` job locally with `--dry_run` and a deliberately failing assertion, and observing the failure occurs with zero `npm publish` invocations. Record the observed behavior in the PR description.

## 3. Streaming smoke at the claimed scale

- [x] 3.1 Raise the workbook size in `scripts/streaming-smoke.cjs` to a multi-sheet workbook with a cell payload materially larger than a spot check (~8 sheets x 4000 rows x 5 cells, ~2.5s locally), staying well inside the release runner's 2-minute step timeout. Do NOT assert a memory bound: measurement showed `write()` heap growth is O(total payload) and flat in sheet count, so no bounded-output property exists to assert. Verify by timing the script and reporting payload size, row count, and elapsed time.
- [x] 3.2 Replace `scripts/streaming-smoke.cjs`'s single-row spot check with an exhaustive fidelity assertion: assert every sheet's row count and every cell value written is read back identical, not just the first row, and assert the payload exceeds a stated floor so the test cannot silently shrink to a spot check. Verify by confirming the script passes on the current build, and that corrupting one expected cell value makes it fail.
- [x] 3.3 Run the streaming smoke test on musl matrix targets instead of skipping it, reusing the Alpine container pattern already used by `Smoke-test musl binding on Alpine`. Verify by confirming the musl condition no longer excludes this step and that the Alpine invocation executes the streaming script successfully.

## 4. Spec corrections

- [x] 4.1 Correct the two `ADR-005` citations in `openspec/specs/streaming-write-incremental/spec.md` to name the streaming write buffering decision, and verify by grepping the file for `ADR-005` and confirming zero matches while the requirement text still scopes the bridge change correctly.
- [x] 4.2 Verify no other live spec cites a decision number whose subject does not match, by listing every `ADR-<n>` reference across `openspec/specs/` and reading the cited decision's title for each. Confirm the four `ADR-028` references in the streaming specs and the three `ADR-005` references now removed are the complete set, and record any further mismatch found as a follow-up rather than expanding this change.
- [x] 4.3 Confirm `openspec validate --strict --specs --no-interactive` passes with 42 specs and that the archived deltas for this change validate clean, verifying the spec edits introduced no structural regression.

## 5. Integration

- [x] 5.1 Run `pnpm verify:build`, `pnpm verify:features`, `pnpm test`, `cargo test`, and `cargo test --features formula-eval` on a feature-enabled build, and confirm every command exits zero — confirming the new checks coexist with the existing suite rather than replacing or skipping it.
- [x] 5.2 Read each corrected requirement in `openspec/specs/` against the workflow it describes and confirm every claim is now literally true of `.github/workflows/release.yml`, naming the step that makes it true. Record any requirement that still overstates its mechanism rather than adjusting the requirement to match.