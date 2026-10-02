# Proposal

## Why

Four claims about the `formula-eval` feature and the generated type declarations are
contradicted by the code, and the contradiction already has a cost: the local development
loop documented in `README.md` (`pnpm build` then `pnpm test`) cannot pass.

PR #62 added `--features formula-eval` to `.github/workflows/ci.yml` but not to the
`build` script in `package.json`. Without the feature, `Worksheet::recalculate()` compiles
to a no-op and `Cell.cachedValue` stays `null`, so the four `recalculate` tests in
`__test__/cached-formula.test.ts` fail with `expected null to be 3`. CI stayed green
because CI builds with the feature. `package.json`'s `build` script has not been touched
since `b6776a5` (#47).

Digging into *why* those tests exist surfaced the wider problem: `openspec/specs/formula-eval/spec.md`
requires the evaluation API to be absent from the public surface when the feature is
disabled, and it is not. The spec describes a build configuration the project does not
produce, so it cannot be used to check anything.

## What Changes

- Add `--features formula-eval` to the `build` and `build:release` scripts in
  `package.json`, so a local build matches the CI and release builds and the documented
  `pnpm build && pnpm test` loop passes.
- Rewrite the formula-eval opt-in requirement. The evaluation surface (`Cell.cachedValue`,
  `Worksheet.recalculate`, `Workbook.recalculate`) is intentionally present in every build;
  what the feature gates is *behavior*, not the surface. `FormulaEvaluator` is already
  internal-only and is unaffected.
- Correct `openspec/specs/package-entrypoint/spec.md`, which describes the generated
  `native.d.ts` transforms as "published type transforms". `native.d.ts` is gitignored
  and is not in the published package; the transform exists so that `pnpm typecheck`,
  which includes `native.d.ts` in `tsconfig.json`, checks the test suite against accurate
  declarations. The sibling requirement about `getCell` overloads being declared in the
  published types is accurate and is left alone — `index.d.ts` is published and is
  hand-maintained, and it does declare all four overload forms.
- Resolve the "not compiled into default builds" wording in `ROADMAP.md` and
  `CHANGELOG.md`. `release.yml` compiles `formula-eval` into every published artifact, so
  npm consumers always get a working `recalculate`; "default build" currently reads as
  though it describes the shipped package, which it does not.

No Rust source changes. No published API changes. Not breaking.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `formula-eval`: the opt-in requirement currently requires the evaluation API to be absent
  from the public surface without the feature, which is false. It will state that the
  surface is always present and the feature gates behavior.
- `package-entrypoint`: one requirement describes the generated type-declaration transform
  as affecting a published artifact. It affects type-checking only; `native.d.ts` is not
  published and `index.d.ts` is hand-maintained.

## Impact

- `package.json` — `build` and `build:release` gain `--features formula-eval`.
- `openspec/specs/formula-eval/spec.md` — first requirement rewritten.
- `openspec/specs/package-entrypoint/spec.md` — one requirement renamed and reworded.
- `ROADMAP.md`, `CHANGELOG.md` — "default build" disambiguated against the release matrix.
- No change to `src/`, to the published runtime surface, or to the release matrix itself.