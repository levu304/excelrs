# Proposal

## Why

`scripts/apply-glue.cjs` is a `napi --pipe` hook with two halves in different states. Its
`.js` half dispatches on `index.js`, a file the build has never emitted — `--js native.js` has
been set since v0.1.0 precisely so napi cannot clobber the hand-maintained entrypoint. That
branch cannot fire, and the `getCell` overloads in the published package are a hand-pasted
copy in a hand-maintained file, not a build product. Its `.d.ts` half, by contrast, is live and
load-bearing: `native.d.ts` carries the `CellValue` discriminated union and the refined `Cell`
setter that only this script produces.

The half that works is the half nothing verifies. `--pipe` has never appeared in
`ci.yml` or `release.yml` — `git log -S'--pipe' -- .github/workflows/` is empty — so a release
artifact's types are built by a different pipeline than a developer's local build. And because
all 16 test suites import the hand-maintained `index.js` rather than the build output, no test
can see the difference: a build with no pipe at all would pass CI. The unreachable branch is
also why requirement `exceljs-parity:191`, which asserts the overloads "SHALL be re-injected
through a build-time hook", reads as satisfied while the mechanism it names is dead.

The same failure shape appears in that capability's other 8 requirements, which mandate the
ongoing accuracy of the hand-maintained `ROADMAP.md` parity matrix. Nothing can check them —
CI runs structural OpenSpec validation, which does not read `ROADMAP.md` — and the matrix has
drifted: four rows still read `targeted` for features that shipped in v1.3.0, contradicting the
spec's own scenario at `spec.md:166`. Both are the same disease: a contract written as prose,
verified against the wrong artifact, decaying silently.

## What Changes

- The `.js` half of `scripts/apply-glue.cjs` is removed. It dispatches on `index.js`, which
  the build has never emitted, so it cannot fire; the `getCell` overloads it claims to inject
  are a hand-pasted copy in a hand-maintained file. The `.d.ts` half is retained: it is live,
  and `native.d.ts` depends on it for the `CellValue` discriminated union and the refined
  `Cell` setter type.
- The CI and release builds gain the `--pipe` flag that only the local `pnpm build` has
  carried since v2.1.0. `git log -S'--pipe' -- .github/workflows/` is empty — the flag was
  never in CI. A published artifact's types are therefore currently produced by a different
  pipeline than a developer's local build.
- A CI check asserts that the generated `native.d.ts` carries the pipe's transforms and that
  the entrypoint still exposes the `getCell` overloads. Nothing currently reads the file the
  build produces: all 16 test suites import the hand-maintained `index.js`, and
  `tsconfig.json`'s `native.d.ts` include is inert because typecheck runs before the build.
- A new `package-entrypoint` capability states the `getCell` guarantee as observable behavior
  — each overload form resolves, the forms are declared in the published types, and CI
  verifies both — rather than as a claim about a build-time re-injection that never ran.
- `exceljs-parity` is reduced from 11 requirements to its 3 historical ones. The 8
  requirements that mandate the ongoing accuracy of the hand-maintained parity matrix, or
  restate its rows as a list, are removed; the v2.0.0 completion record and its exclusion
  list are retained.
- `spec-integrity` gains two requirements: one that a requirement naming an enforcement
  mechanism name a resolvable target, and one that an unenforceable value is not stated as a
  standing invariant.
- The `openspec/config.yaml` project context is corrected, since it currently describes the
  pipe as re-injecting overloads into `index.js`.

## Capabilities

### New Capabilities
- `package-entrypoint`: The ExcelJS-compat `getCell` overloads resolve in every call form, are
  declared in the published type declarations, and are verified by the pipeline rather than
  asserted by a requirement that describes an unused mechanism.

### Modified Capabilities
- `exceljs-parity`: Retire the 8 unenforceable or list-restating requirements governing the
  hand-maintained parity matrix, and remove the misfiled `getCell` requirement whose asserted
  build-time re-injection has never run.
- `spec-integrity`: Require that a requirement naming an enforcement mechanism name a
  concrete resolvable target, and that an unenforceable value is not stated as a standing
  invariant.

## Impact

**Build and packaging**
- `scripts/apply-glue.cjs` — the `index.js` branch and the `index.d.ts` marker machinery are
  removed; the `native.d.ts` transform path is unchanged.
- `.github/workflows/ci.yml` — the build step gains `--pipe`.
- `.github/workflows/release.yml` — both build invocations (musl and non-musl) gain `--pipe`.
- `package.json` — **unchanged**. `--js native.js --dts native.d.ts` is deliberate
  (`CHANGELOG.md:812`: it preserves the hand-maintained `index.js` across builds) and correct.
- `index.js`, `index.d.ts`, `native.js`, `native.d.ts` — all unchanged in role and provenance.

**Specs**
- `openspec/specs/exceljs-parity/spec.md` — 11 requirements → 3.
- `openspec/specs/package-entrypoint/spec.md` — new.
- `openspec/specs/spec-integrity/spec.md` — two requirements added.

**Docs**
- `openspec/config.yaml` `context` — corrected to describe what the pipe actually does.
- `ROADMAP.md` — **not** modified. The four stale `targeted` rows remain; this change removes
  the contract that made them a violation rather than silently rewriting the file.
  Reconciling the matrix is follow-up work, and is the wrong fix for a place that rots.

**Not breaking for consumers**
- No change to the published entrypoint, `main`/`types` fields, exported API, or generated
  types. A release artifact's types become identical to a local build's, which is a fix.
