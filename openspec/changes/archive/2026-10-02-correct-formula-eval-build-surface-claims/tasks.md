# Tasks

## 1. Local build scripts

- [x] 1.1 Add `--features formula-eval` to the `build` script in `package.json` and verify `grep '"build":' package.json` shows the flag
- [x] 1.2 Add `--features formula-eval` to the `build:release` script in `package.json` and verify `grep '"build:release":' package.json` shows the flag
- [x] 1.3 Run `pnpm build` then `pnpm test` and verify the suite passes with no failures — specifically the four `recalculate` tests in `__test__/cached-formula.test.ts` that currently fail with `expected null to be 3` on a default build

## 2. Specification corrections

- [x] 2.1 Apply the MODIFIED delta to `openspec/specs/formula-eval/spec.md`: replace the "absent from the public surface" requirement with the present-but-inert requirement, retaining the "Evaluation API absent without feature" scenario per `design.md` and noting in the spec that the title is narrower than its contents
- [x] 2.2 Apply the two ADDED requirements to `openspec/specs/formula-eval/spec.md` — "Published artifacts compile in formula evaluation" and "The local build matches the pipeline's feature set" — verifying `openspec validate --strict` reports no INFO finding about archive refusing the delta
- [x] 2.3 Apply the RENAMED + MODIFIED delta to `openspec/specs/package-entrypoint/spec.md`, renaming the requirement to drop "published" and adding the scenario "The transform does not reach a consumer"
- [x] 2.4 Run `openspec validate --strict --specs --changes` and verify every item passes with no INFO or ERROR

## 3. Documentation

- [x] 3.1 Update the formula-evaluation row in `ROADMAP.md` to state that the feature is not in the crate's default Cargo feature set, and that published release artifacts do include it — verify the row still uses a status the `ROADMAP.md` legend defines
- [x] 3.2 Update the corresponding "not compiled into default builds" sentence in `CHANGELOG.md` to name the Cargo default rather than the shipped package, and verify no unqualified "default build" claim about the published artifact remains
- [x] 3.3 Run `pnpm verify:roadmap` and verify it passes

## 4. Integration verification

- [x] 4.1 Confirm `git diff --stat -- src/` is empty and verify no Rust source was modified
- [x] 4.2 Run `pnpm typecheck` and `pnpm verify:build` and verify both pass
- [x] 4.3 Run `cargo test --features formula-eval` and verify the suite passes
- [x] 4.4 Confirm the documented loop in `README.md` (lines 197-199) is accurate as written now that `pnpm build` enables the feature, and verify no additional documented step is required