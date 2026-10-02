# Design

## Context

See `proposal.md` — Why, for motivation. This section records only the current state that
shapes the approach.

The package has three JavaScript files with distinct provenance, which is easy to mistake for
duplication:

| File | Provenance | In git | Produced by the build |
| --- | --- | --- | --- |
| `native.js` | napi-generated | yes | yes — `--js native.js` |
| `native.d.ts` | napi-generated, then pipe-transformed | **no** (`.gitignore:20`) | yes — `--dts native.d.ts` |
| `index.js` | hand-maintained | yes | no |
| `index.d.ts` | hand-maintained | yes | no |

`--js native.js` is deliberate. `CHANGELOG.md:812` (v0.1.0) records it as the means of
"preserving `index.js` across builds" — napi is pointed at a disposable filename so it cannot
clobber the hand-maintained entrypoint. `index.d.ts` is likewise hand-maintained so that
`pnpm typecheck` works before any build; `index.d.ts:3-4` and `ci.yml:48` (typecheck) running
before `ci.yml:52` (build) both depend on that.

`scripts/apply-glue.cjs` is a `napi --pipe` hook with two halves that are not equally alive:

- **`.d.ts` half — live and load-bearing.** `apply-glue.cjs:101` matches `native.d.ts`, and
  `:128` adds class-body refinements for it. `native.d.ts:22` currently contains
  `set value(val: CellValueInput | string | number | boolean | Date | null)`, which exists
  nowhere else. The `CellValue` discriminated union and `CellValueInput` type are produced
  only here.
- **`.js` half — dead.** `:83` matches `index.js`, which the build never emits. It cannot fire.
  The `getCell` overloads in `index.js` are a committed hand-pasted copy, not a build product.

Neither `ci.yml` nor `release.yml` has ever passed `--pipe` (`git log -S'--pipe' --
.github/workflows/` is empty). CI builds `native.d.ts` without the union transform, and nothing
detects it: the 16 test files import `../index`, never `../native`, and `tsconfig.json`'s
`native.d.ts` include matches nothing on a clean checkout because typecheck precedes the build.

## Goals / Non-Goals

**Goals**
- Make the `.d.ts` half of the pipe run wherever `native.d.ts` is built, so a release artifact's
  types match a local build's types.
- Add a check that fails when the pipe's output is missing, so a regression in it is caught by
  CI rather than by a consumer's typecheck.
- Remove the dead `.js` half of the pipe, which is what makes requirement
  `exceljs-parity:191` read as satisfied while describing a mechanism that never ran.
- Restate the `getCell` guarantee as observable behavior under `package-entrypoint`.
- Retire the 8 `exceljs-parity` requirements that mandate the ongoing accuracy of a
  hand-maintained table nothing can check.

**Non-Goals**
- Changing which file napi emits. `--js native.js` is correct and deliberate; changing it would
  clobber hand-maintained files.
- Regenerating, mirroring, or deduplicating `index.js` / `index.d.ts`. They are maintained by
  hand on purpose.
- Reconciling the four stale `targeted` rows in `ROADMAP.md`'s parity matrix. This change
  retires the contract that made them a violation; rewriting the file is separate work and
  would treat the symptom.
- Any change to the public API or to what consumers import.

## Decisions

### D1: Keep the pipe's `.d.ts` half; delete only its `.js` half

**Decision.** Remove the `basename === 'index.js'` branch at `apply-glue.cjs:83-95` and the
`DTS_MARKER` / `DTS_GLUE` machinery that exists only to mark `index.d.ts`. Leave the
`native.d.ts` transform path intact.

**Rationale.** The two halves are independent. The `.js` branch is unreachable and its output
is duplicated by hand. The `.d.ts` branch is the sole producer of the `CellValue` union and the
`Cell` setter refinement that `native.d.ts` carries today. Deleting the whole script — the
obvious reading of "the hook never fired" — would silently revert those types.

**Alternatives considered.**
- *Delete the script entirely.* Rejected: it would remove the only source of the
  `CellValueInput` union and the `set value` refinement, regressing generated types. The
  script's `.d.ts` half has demonstrably run and its output is in the current build.
- *Repoint the build to `--js index.js`.* Rejected: `--js native.js` is the mechanism that
  preserves the hand-maintained `index.js` (`CHANGELOG.md:812`), and `.gitignore:20` /
  `index.d.ts:3-4` show the `native.d.ts` / `index.d.ts` split is load-bearing for
  pre-build typecheck. This would delete the shield, not fix the hole.
- *Leave the dead branch in place.* Rejected: it is the reason `exceljs-parity:191` appears
  satisfied. `spec-integrity`'s new requirement treats a mechanism aimed at a file the build
  does not emit as a failed requirement, so leaving it would keep the corpus in violation of
  its own new rule.

### D2: Add `--pipe` to the CI and release builds

**Decision.** Pass `--pipe "node scripts/apply-glue.cjs"` alongside `--js native.js --dts
native.d.ts` in `ci.yml:52` and in both `release.yml` build invocations (`:105`, `:110`).

**Rationale.** The local `pnpm build` has carried `--pipe` since `f37cc2c`; the CI step was
written independently and never copied the flag. A release binary's types are therefore not
produced by the same pipeline as a developer's local build.

**Safety.** The `.d.ts` branch is idempotent by construction: the `CellValue` replacement runs
*before* the `DTS_MARKER` early return (`apply-glue.cjs:97-99` states this deliberately), and
each transform is guarded by a `content.includes` check. The `.js` branch being removed (D1)
means no remaining branch writes to a hand-maintained file.

**Alternatives considered.**
- *Extract a shared build script* both workflows invoke, so the flags cannot drift again.
  Rejected as scope: the three call sites are visible and the drift is a one-line omission,
  not a structural problem. Worth a follow-up if a fourth appears.
- *Move the transforms out of the pipe and into the Rust side.* Rejected: the union is a
  TypeScript-level concern napi cannot express, which is why the pipe exists.

### D3: Add a CI check that the pipe's output is present

**Decision.** Add a step after the CI build that asserts the generated `native.d.ts` contains
the `CellValue` union and the refined `Cell` setter, and that `index.js` still exposes the
`getCell` overloads.

**Rationale.** Today nothing reads the file the build produces. The 16 test suites import
`../index`; the type-level test imports `../index` too; `tsconfig.json`'s `native.d.ts`
include is inert because typecheck precedes the build. A pipe that is never exercised cannot
regress visibly — which is how the `.js` half stayed dead from v2.1.0 to v2.9.1 without
anyone noticing.

This is the one genuinely new mechanism in the change, and it is added because the
alternative — an unverified transform that every published type depends on — is worse. It is
scoped to assert presence of specific transforms, not to diff the file against a golden copy,
so it does not become a new place to rot.

**Alternatives considered.**
- *Run the vitest suite against `native.js` instead of `index.js`.* Rejected as a large,
  separate change: it would require `native.js` to carry the `getCell` glue, which is the
  opposite of the D1 decision.
- *Assert `native.d.ts` matches a committed golden file.* Rejected: napi output is
  version-sensitive and would make every napi bump a diff to review, reintroducing exactly
  the restated-inventory problem this change exists to remove.

### D4: Move the `getCell` guarantee to `package-entrypoint`, restated as behavior

**Decision.** Remove `exceljs-parity:191`. Add requirements to the new `package-entrypoint`
capability covering: the overloads resolve in each call form, the overloads are declared in
the published types, and CI verifies both the generated types and the entrypoint.

**Rationale.** The current requirement is misfiled (an FFI/build-pipeline guarantee inside a
feature-parity capability) and false (it asserts a `.js` re-injection that has never run). Its
scenarios test only that the overloads resolve at runtime — which passes, because `index.js` is
hand-maintained — so it certified a mechanism that was dead. Restating as observable behavior
means the guarantee survives a change in how the glue is delivered, which is the point.

**Alternatives considered.**
- *Keep `:191` and just fix the script.* Rejected: the requirement's own text would still
  describe an implementation that does not exist, and `design-record`'s standing requirement
  would flag it.

### D5: Retire the 8 matrix requirements rather than enforce them

**Decision.** Remove the 8 `exceljs-parity` requirements that mandate the ongoing accuracy of
`ROADMAP.md`'s matrix or restate its rows. Retain the 3 historical ones (v2.0.0 completion,
v2.0.0 exclusions, and the status-authority rule folded into the v2.0.0 record's migration
note).

**Rationale.** CI runs `openspec validate --strict`, which validates spec *structure* and does
not read `ROADMAP.md`. There is no oracle, so the obligation decayed silently: four rows still
read `targeted` for features that shipped in v1.3.0, contradicting this capability's own v1.3.0
scenario. The porting program the requirements governed was declared complete at v2.0.0 and
survives as a historical record.

This follows `spec-integrity`'s existing rule — *"prefer deleting a place that can rot over
adding a mechanism to check it"* — and the new requirement added in
`specs/spec-integrity/spec.md`.

**Alternatives considered.**
- *Write a script that greps `src/` for the expected symbols and fails if a matrix row is
  wrong.* Rejected: a hand-maintained symbol allowlist is a new place to rot, just smaller.
- *Fix the four stale rows and keep the contract.* Rejected: it treats this instance and leaves
  the class in place; the next release drifts again silently.

### D6: Correct the `openspec/config.yaml` context line

**Decision.** Update the `context` block, which states *"`index.js` is a hand-maintained JS
glue layer for method overloading"* — still true after this change, but the same block's
description of the pipe needs to match reality: the pipe transforms generated types only, and
CI/release now run it.

**Rationale.** `config.yaml` `context` is read as authoritative by every artifact-writing
workflow. Leaving it describing a `.js` re-injection would propagate the false model into the
next change's proposal.

## Risks / Trade-offs

- **A contributor workflow outside this repo may run `napi build` and expect `index.js` to be
  regenerated.** Deleting the `.js` branch removes the only code path that could ever write
  that file. → Mitigated by `index.js` being hand-maintained since v0.1.0 and never having been
  a build output; but this is the one risk not verifiable from inside the repository, and it is
  recorded as an open question rather than assumed away.
- **Adding `--pipe` to CI could fail the build if the script errors on a platform it has never
  run on.** The script is plain Node with `fs`/`path` only and no platform branches, but it has
  only ever run on maintainer machines. → The D3 check fails loudly and specifically if the
  transform is absent, which is the intended signal; the first CI run is the real test.
- **D5 removes obligations rather than satisfying them, so nothing prevents future matrix
  drift.** Accepted deliberately. The matrix remains as reader-facing documentation, and
  `config.yaml`'s authority order already ranks `CHANGELOG.md` and `src/` above it. Adding a
  guard would recreate the rot under a new name.
- **The scope spans build config, CI, and three spec files.** → Each group in `tasks.md` lands
  its own verification, so a partial landing is detectable rather than silently inconsistent.

## Migration Plan

No consumer-visible migration. `package.json` `main` still resolves to `index.js`; the
published entrypoint, exports, and types are unchanged. The change alters only how
`native.d.ts` is produced in CI/release and removes an unreachable code path.

Rollback is a revert of the `--pipe` additions to the two workflows and the D3 check; the
`apply-glue.cjs` deletion and the spec edits are independent of that and can be reverted
separately.

## Open Questions

- Does any workflow outside this repository (a contributor script, a downstream fork's release
  process) depend on `napi build` regenerating `index.js`? Not answerable from this repo; the
  D1 deletion assumes not. Resolving it does not change the specs or the task breakdown — if the
  answer is yes, D1 becomes D1-prime (repoint the branch instead of deleting it), which is a
  local edit to one task.
