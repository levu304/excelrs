# Design

## Context

See `proposal.md` — Why for motivation. What constrains the approach:

**The evaluation surface cannot be cfg-gated without breaking type-checking.**
`tsconfig.json` includes `native.d.ts` alongside `__test__/**/*.ts`. napi generates
`native.d.ts` from the `#[napi]` items present in that build. `Worksheet::recalculate`
and `Cell::cachedValue` are `#[napi]` and are not behind `#[cfg]`
(`src/model/worksheet.rs:249`, `src/model/cell.rs:502`); only their bodies are.

If the surface were cfg-gated as `openspec/specs/formula-eval/spec.md` currently
requires, a build without the feature would drop `recalculate()` from `native.d.ts`, and
the four `recalculate` tests would fail **at type-check time** with "property does not
exist" rather than failing an assertion. PR #62 chose present-but-inert for this reason.

**`FormulaEvaluator` is already absent everywhere.** It is a plain Rust struct at
`src/formula/bridge.rs:144` with no `#[napi]` attribute; it appears in no generated JS
declaration. The requirement names it alongside two symbols that *are* exposed, which is
part of why the requirement reads as unsatisfiable.

**`native.d.ts` is not published.** `npm pack --dry-run` lists `index.js`, `native.js`,
`index.d.ts`, `dts-header.d.ts`, `enums.d.ts` — no `native.d.ts` (it is gitignored at
`.gitignore:20`). `package.json` sets `"types": "index.d.ts"`, and `index.d.ts:3-4` says
it inlines its cell-value types specifically to avoid importing the generated file. The
generated declarations are consumed only by `tsc`.

**The release workflow already enables the feature.** `release.yml:105,111` pass
`--features formula-eval` on every published target, so npm consumers receive working
recalculation.

## Goals / Non-Goals

**Goals:**

- Make the local build enable the same features CI and release enable.
- Make `formula-eval` and `package-entrypoint` describe what the code does.
- Disambiguate "default build" so it names the Cargo default, not the shipped package.

**Non-Goals:**

- No Rust source changes. The present-but-inert surface is correct and stays.
- No change to the release matrix, the feature's runtime behavior, or the published API.
- No new enforcement mechanism. `verify-build-output.cjs` already fails when the
  transforms are absent; the local build fix is sufficient and adding a feature-flag
  checker would be a second inventory to maintain.

## Decisions

**Correct the spec rather than cfg-gate the surface.**
The evidence is one-directional: `tsconfig` includes the generated declarations, so
gating the surface converts four runtime assertion failures into type-check failures, and
it contradicts a deliberate choice in #62. *Alternative considered:* gate `cachedValue` and
`recalculate` behind `#[cfg_attr]` and add a default-build type declaration shim — rejected
as strictly worse failure behavior for the same correctness gain that is already achieved by
inertness.

**Retain the scenario title "Evaluation API absent without feature".**
`openspec validate` refuses a MODIFIED requirement that drops a scenario the current spec
has, so the scenario must survive archive. Its title is now narrower than its contents: the
evaluator implementation is genuinely absent, while the callable surface is present. The
body states both facts. *Alternative considered:* REMOVE the requirement and ADD a
replacement under the same name — rejected because it discards requirement history for a
title that is imprecise rather than false. Renaming the scenario title to match its scope is
left as a follow-up, since archive cannot rename a scenario.

**State the release behavior as an invariant over `release.yml`, not as a list.**
The new "Published artifacts compile in formula evaluation" requirement names
`.github/workflows/release.yml` as the artifact it is checked against rather than
restating which targets exist. Restating the matrix is how the `exceljs-parity` matrix
rotted — see `openspec/specs/exceljs-parity/spec.md` — so the requirement asks for
agreement with the workflow instead.

**Do not extend `verify-build-output.cjs` to check the feature flag.**
The build scripts are the single source of truth for the local feature set and are edited
in the same commit as this spec change. A checker that reads `package.json` to confirm
`package.json` contains a flag guards nothing.

## Risks / Trade-offs

**`pnpm build` becomes slower for everyone** → the feature was already compiled for CI and
release, so this only affects local builds, and they previously produced an artifact the
test suite could not use. If local build time becomes a complaint, the alternative is a
`build:no-formula` script rather than reverting the default.

**The retained scenario title is narrower than its contents** → flagged above and in the
task list as an explicit follow-up rather than left for a reader to discover.

**A consumer building from source still gets inert recalculation** → this is pre-existing,
documented in the rewritten requirement, and does not affect npm consumers. Making the
source-build default include the feature is a Cargo `default = ["formula-eval"]` change and
is deliberately excluded; it would change what "opt-in" means.