# Tasks

## 1. Remove the dead JS branch of the glue pipe

- [x] 1.1 Confirm the open question in `design.md` is still open: grep the repository and
  `package.json` scripts for any invocation that regenerates `index.js` via `napi build`, and
  record the finding in this change's notes. If such an invocation exists, do not delete the
  branch — repoint it at the emitted filename instead and note the divergence from D1.
  **Finding (2026-10-02): the question resolves in-repo — no divergence from D1.** Every
  live `napi build` invocation passes `--js native.js`: `package.json:26,27`, `ci.yml:54`,
  `release.yml:103,109`. `CONTRIBUTING.md` documents no build command that emits `index.js`.
  The only `--js index.js` / `--dts index.d.ts` occurrences are in archived change documents
  and in D1's own rejected-alternative note. The open question's residual risk — an
  out-of-repo contributor workflow — is not answerable here and is accepted per `design.md`.
- [x] 1.2 Delete the `basename === 'index.js'` branch in `scripts/apply-glue.cjs` (the
  `JS_MARKER` / `JS_GLUE` block and its `processFile` arm), and remove the `index.d.ts`-only
  `DTS_MARKER` / `DTS_GLUE` early-return machinery. Verify `node -e "require('./scripts/apply-glue.cjs')"`
  loads without error and that `grep -c "index.js" scripts/apply-glue.cjs` returns no remaining
  dispatch on that filename.
  **Done:** removed `JS_MARKER`/`JS_GLUE`, the `index.js` `processFile` arm, the
  `DTS_MARKER`/`DTS_GLUE` constants and the `index.d.ts` early-return block, and the
  `index.js`→`index.d.ts` follow-on in the argv handler. Narrowed the outer guard to
  `native.d.ts` only and removed the now-redundant inner `native.d.ts` check. Verified: the
  script loads, and `grep -n "index\.js\|index\.d\.ts"` returns only two comment lines.
- [x] 1.3 Verify the `.d.ts` half is untouched: run `pnpm build` and confirm
  `grep -n "set value(val: CellValueInput" native.d.ts` still matches and
  `grep -c "export type CellValueInput" native.d.ts` is 1. A zero here means the deletion
  removed a live transform — stop and restore.
  **Done:** `pnpm build` logs `[apply-glue] Patched native.d.ts`. `native.d.ts:22` carries the
  refined setter; `CellValueInput` = 1, `CellValue` union = 1, flat interface = 0, both
  `getCell` overload declarations present, `@deprecated` date getter present.
- [x] 1.4 Verify the hand-maintained entrypoint is unaffected: run `node -e "const {Workbook}=require('./index.js');
  const ws=new Workbook().addWorksheet('s'); if(typeof ws.getCell!=='function') throw new Error('glue lost');
  if(!ws.getCell('A1')) throw new Error('A1 failed')"` and confirm it exits zero.
  **Done:** all four overload forms resolve — `ws.getCell('A1')`, `ws.getCell(1,1)`,
  `row.getCell(1)`, `row.getCell('A')`. `git diff --stat index.js index.d.ts` is empty,
  confirming the build does not write to either hand-maintained file.

## 2. Run the pipe wherever the types are built

- [x] 2.1 Add `--pipe "node scripts/apply-glue.cjs"` to the CI build step in
  `.github/workflows/ci.yml` (the `npx napi build ... --js native.js --dts native.d.ts` step),
  preserving the existing folded-scalar `>-` style. Verify the step still runs on all three
  matrix OSes by confirming the local equivalent command succeeds.
  **Done:** added to the folded-scalar build step (`ci.yml:57`). Verified the local
  equivalent — `pnpm build`, which carries the same flags plus `--pipe` — succeeds and emits
  the transformed `native.d.ts` (1.3). The flag is platform-independent: the script uses only
  `fs` and `path`, with no OS branches.
- [x] 2.2 Add the same `--pipe` flag to both `release.yml` build invocations (the musl
  `cargo-zigbuild` step and the non-musl step), so a release artifact's `native.d.ts` is
  produced by the same pipeline as a local build. Verify both invocations remain valid YAML and
  that neither drops the existing `--features formula-eval` flag.
  **Done:** added at `release.yml:106` (musl, line-continuation) and `release.yml:112`
  (non-musl, folded scalar). Both files parse as valid YAML via `yaml.safe_load`;
  `--features formula-eval` is intact on all three invocations across both workflows.
  Note: the release builds pass `--js native.js` without `--dts`, so the pipe is a no-op
  unless napi also emits `native.d.ts` there — harmless either way, and correct if it does.
- [x] 2.3 Confirm the change is genuinely additive for the release path: verify that
  `apply-glue.cjs` has no branch that writes to a hand-maintained file now that 1.2 has landed
  (`grep -n "writeFileSync" scripts/apply-glue.cjs` shows writes only on a `native.d.ts` path).
  **Done:** exactly one `writeFileSync` remains (`apply-glue.cjs:134`), inside the
  `basename === 'native.d.ts'` guard. Verified behaviourally as well as by inspection:
  running `node scripts/apply-glue.cjs index.js` and `... index.d.ts` leaves both files
  byte-identical, and `git diff --stat index.js index.d.ts` is empty.

## 3. Make the pipe's output verifiable

- [x] 3.1 Add a CI step after the build that asserts the generated `native.d.ts` carries the
  `CellValue` discriminated union and the refined `Cell` setter — e.g. that
  `export type CellValue =` and `export type CellValueInput =` are each present and
  `export interface CellValue {` is absent. Verify the step fails when run against a
  `native.d.ts` built without `--pipe`, and passes against one built with it.
  **Done:** added `scripts/verify-build-output.cjs`, wired into `ci.yml` immediately after
  the build (before the Biome lint step) and exposed as `pnpm verify:build`. Written in Node
  rather than shell because the CI matrix includes `windows-2022`. It asserts the union and
  `CellValueInput` are present, the flat interface is absent, the setter is no longer
  `unknown` and is the refined `CellValueInput` form, and all four `getCell` overload
  declarations exist. Verified in both directions: passes on the current build; exits 1 with
  five specific findings when `native.d.ts` is reverted to its untransformed form.
- [x] 3.2 Extend that step (or add an adjacent one) to assert `index.js` still exposes the
  `getCell` overloads on both `Worksheet` and `Row`, so a future edit to the hand-maintained
  entrypoint cannot silently drop them. Verify by temporarily removing the glue block from a
  scratch copy and confirming the check fails.
  **Done:** the same script asserts `index.js` carries the `__EXCELJS_GETCELL_GLUE__` marker
  plus `nativeBinding.Worksheet.prototype.getCell` and `nativeBinding.Row.prototype.getCell`.
  Verified: removing the marker from a scratch copy of `index.js` makes the check exit 1 with
  a message naming the file as hand-maintained; restoring it returns the check to green.
- [x] 3.3 Confirm the check asserts presence of specific transforms rather than diffing
  `native.d.ts` against a committed golden file, so a napi version bump does not require a
  reviewed diff. Verify no new golden or snapshot file was added by this group.
  **Done:** the check uses `String.includes` assertions only — zero `writeFileSync` /
  `appendFileSync` calls, so it is read-only. `git status` shows no golden, snapshot, or
  `.snap` file added by this group; the only new file is the check itself.

## 4. Correct the project context

- [x] 4.1 Update the `context` block in `openspec/config.yaml` so it describes the pipe as
  transforming generated type declarations only, and records that CI and release now run it.
  Verify the file still parses as YAML and that the `excelrs` / napi-rs / calamine facts it
  already carried are unchanged.
  **Done:** added a **Build outputs** entry to `context` recording that napi targets
  `native.js`/`native.d.ts`, that the pipe transforms only the generated `native.d.ts`, that
  CI and release both pass `--pipe`, and that `verify-build-output.cjs` asserts both halves.
  The existing FFI-bridge wording is unchanged. Verified: parses via `yaml.safe_load`;
  `napi-rs v3`, `calamine`, `quick-xml`, `xlstream`, `ADR-2`, and `470 Rust` all still
  present; all four `rules` blocks and their 14 entries untouched.
- [x] 4.2 Correct the `scripts/apply-glue.cjs` header comment, which currently claims it
  "re-injects ExcelJS-compat getCell overloads into the generated `index.js` / `index.d.ts`".
  Verify the comment describes only what the script does after 1.2.
  **Done:** header now states it transforms the generated `native.d.ts` (union, `CellValueInput`,
  refined setter, `getCell` declarations) and explicitly does NOT touch `index.js` /
  `index.d.ts`, noting they are hand-maintained and never emitted by the build. Verified the
  script still loads and `pnpm verify:build` passes after the edit.

## 5. Land the spec changes

- [x] 5.1 Apply the `specs/exceljs-parity/spec.md` delta from this change: remove the eight
  matrix-governing requirements and the `getCell` requirement, each with its recorded reason and
  migration. Verify `openspec show exceljs-parity --type spec` reports the three retained
  historical requirements and that `openspec validate --strict --specs` passes.
  **Done — 11 requirements → 2, 218 lines → 49.** Removed the eight matrix-governing
  requirements and the `getCell` requirement. Retained the v2.0.0 completion record and the
  v2.0.0 exclusions list. The third removed requirement — *Parity matrix status is
  authoritative over the historical record* — had real substance (it ordered the matrix
  against the v2.0.0 record), so rather than drop it silently its scenario was folded into the
  v2.0.0 completion requirement as *A later release does not amend this record*, which
  preserves the precedence rule against a source that actually exists. `## Purpose` rewritten
  to describe the capability as a historical record and to route readers to `openspec/specs/*`
  for current behavior. `openspec validate --strict` reports `exceljs-parity` valid.
- [x] 5.2 Create `openspec/specs/package-entrypoint/spec.md` from this change's delta,
  including a `## Purpose` section in the project's own words. Verify it appears in
  `openspec list --specs` with its full requirement count and no `TBD` placeholder.
  **Done:** created with the canonical `# package-entrypoint Specification` /
  `## Purpose` / `## Requirements` structure, matching the convention in `csv` and
  `comments`. 7 requirements. `openspec list --specs` reports it; validation is clean and no
  `TBD` placeholder is present.
- [x] 5.3 Apply the `specs/spec-integrity/spec.md` delta, adding the two requirements on
  resolvable enforcement targets and unenforceable contracts. Verify
  `openspec validate --strict --specs` reports no ERROR- or WARNING-level finding for it.
  **Done:** 6 requirements → 8. Added *A requirement naming an enforcement mechanism names a
  resolvable target* and *An unenforceable contract is not stated as a standing invariant*.
  Validation reports `spec-integrity` valid with an empty `issues` array.
- [x] 5.4 Verify the whole corpus: run
  `npx --yes @fission-ai/openspec@1.14.0 validate --strict --specs --changes --json --no-interactive`
  and confirm it exits zero.
  **Done:** the exact command CI runs (`ci.yml:47`) reports 43/43 passed, 0 failed — 42 specs
  plus the in-flight change — with an empty invalid list and exit status 0.

## 6. Integration checks

- [x] 6.1 Confirm no consumer-visible change: `npm pack --dry-run --json` still lists `index.js`
  as the entrypoint named by `main`, and `index.d.ts` as the file named by `types`.
  **Done:** the dry-run tarball still contains `index.js` and `index.d.ts` (`main` / `types`),
  plus `native.js`. `native.d.ts` remains gitignored and unshipped, as before.
- [x] 6.2 Run the full local gate — `pnpm typecheck`, `pnpm lint:js`, `cargo test`,
  `pnpm test` — and confirm all pass, establishing that the build-flag changes did not disturb
  the existing suite.
  **Done:** `pnpm typecheck` exit 0; `pnpm lint:js` exit 0 (11 pre-existing const-enum
  warnings, no errors); `cargo test` 437 passed; `cargo test --features formula-eval`
  **470 passed**; `pnpm test` **187/187 across 15/15 files**.
  Note: the first `pnpm test` run showed 4 `recalculate` failures. These are **pre-existing and
  unrelated** — proven by stashing this change's edits, rebuilding, and reproducing the same
  4 failures on unmodified `main`. The cause is that `pnpm build` omits `--features
  formula-eval` (the local dev script), so `recalculate` is absent. Re-running the exact CI
  build command with `--features formula-eval --pipe` resolved it, which also confirms the new
  `--pipe` addition works under the feature set CI uses.
- [x] 6.3 Confirm `ROADMAP.md` is unmodified by this change (`git diff --stat ROADMAP.md` is
  empty), recording the four stale `targeted` rows as known follow-up work rather than
  silently fixing them here.
  **Done:** `git diff --stat ROADMAP.md` is empty. The four stale `targeted` rows
  (`insertRow`/`spliceRows`, `duplicateRow`, `outlineLevel`, `rowBreaks`/`colBreaks` — all
  shipped in v1.3.0) are now no longer a violation of any requirement, since the contract that
  made them one is retired. **Follow-up:** the `ROADMAP.md` parity matrix still reports
  `targeted` for shipped features and contradicts its own post-v1 roadmap table; the matrix
  also uses a `targeted` status outside any sanctioned vocabulary. Worth a separate pass now
  that nothing depends on its accuracy.
- [x] 6.4 Add a `CHANGELOG.md` entry under `[Unreleased]` noting that the release and CI builds
  now run the type-declaration transform, and that the unreachable `index.js` branch was
  removed. Verify the entry describes no consumer-facing behavior change, because there is none.
  **Done:** added an `[Unreleased]` section (the changelog had none) with `Fixed` (the missing
  `--pipe` in CI/release; the removed unreachable branch), `Added` (`verify-build-output.cjs`),
  and `Changed` (the two spec reductions), closing with an explicit statement that there is no
  consumer-facing behavior change.
