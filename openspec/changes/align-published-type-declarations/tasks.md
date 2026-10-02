# Tasks

## 1. Exported enums become importable by transpiling consumers

- [x] 1.1 Remove the `const` modifier from all eleven `export declare const enum` declarations in `index.d.ts`, matching the form `napi build --no-const-enum` emits. Verify `pnpm lint:js` reports `Found 0 warnings`, down from 11, and that no enum's member list changed
- [x] 1.2 Add a vitest case that writes a scratch consumer importing an exported enum and type-checks it with `isolatedModules` enabled, asserting no `TS2748`. Verify the case fails with `TS2748` when 1.1 is reverted and passes as committed — a check that cannot fail proves nothing
- [x] 1.3 Confirm the enums still resolve at runtime after the form change, since a plain enum depends on the entrypoint supplying the object. Verify a vitest case asserting an imported enum member equals its expected string value still passes

## 2. Published export surface covers the runtime

- [x] 2.1 Declare `SheetState` in `index.d.ts` as a plain enum with the values the binding exports. Verify `pnpm typecheck` passes and a scratch consumer compile that previously failed with `TS2305: has no exported member 'SheetState'` now compiles
- [x] 2.2 Rewrite `WorksheetState` as a template-literal type derived from `SheetState` rather than a hand-written union of its values. Verify a scratch consumer compile assigning the bare literals — `ws.state = 'visible'` and `const s: WorksheetState = 'hidden'` — still succeeds, since aliasing the alias directly to the enum breaks both with `TS2322`
- [x] 2.3 Add a `CHANGELOG.md` entry stating that exported enums are importable under isolated modules and that `SheetState` is now declared, keeping `WorksheetState` as an ExcelJS-compatible alias. Verify the entry names no enum or export that the change does not actually touch

## 3. Published and generated declarations are checked for agreement

- [x] 3.1 Add a coverage assertion to `scripts/verify-build-output.cjs` that reads `native.d.ts` and `index.d.ts` through the TypeScript compiler API and fails when the generated declarations export a name the published ones omit. Verify it FAILS on the tree as it stands, naming `SheetState` — an assertion that cannot fail is not a check
- [x] 3.2 Add an assertion comparing whether each enum both files declare is declared `const`. Verify it FAILS when a published enum declaration is temporarily made `const`, naming that enum
- [x] 3.3 Verify the coverage assertion does not fire on the additional published alias `WorksheetState`, confirming the comparison is one-directional and needs no allowlist
- [x] 3.4 After 1.1 and 2.1 land, verify `pnpm build && pnpm verify:build` passes, that the check still runs in `.github/workflows/ci.yml` and `.github/workflows/release.yml` without a pipeline edit, and that a keyword-regex parse is not used anywhere in the new code

## 4. Redundant hand-maintained name list is deleted

- [x] 4.1 Capture the current runtime export key set from `index.js`, delete the 22 `module.exports.<Name> = nativeBinding.<Name>` assignments that follow `module.exports = nativeBinding`, and verify the export key set is identical before and after and that no entry was a rename
- [x] 4.2 Verify `pnpm test` passes unchanged, since the deleted assignments were no-ops, and confirm no remaining reference in `index.js` or `__test__/` depended on them being written out explicitly

## 5. Integration verification

- [x] 5.1 Run the full gate on a clean tree: `pnpm typecheck`, `pnpm lint:js`, `pnpm test`, `pnpm build && pnpm verify:build`, `cargo clippy --features formula-eval -- -D warnings`, `cargo test`, `cargo test --features formula-eval`, and `openspec validate --strict --specs --changes`. Verify every one passes with nothing skipped
- [x] 5.2 Verify the change is a pure type-surface and tooling change by confirming the git diff touches no file under `src/` and no `.node` artifact is staged
