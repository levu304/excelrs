# Proposal

## Why

The package publishes a hand-maintained `index.d.ts` alongside a build-generated
`native.d.ts`, and nothing compares them. They have already diverged in three ways that
the existing checks cannot see, because `scripts/verify-build-output.cjs` reads only the
generated file.

The sharpest divergence is a correctness bug for consumers: eleven enums are declared
`declare const enum` in the published declarations, contradicting the build's deliberate
`--no-const-enum --runtime-string-enum` flags. A consumer compiling with
`isolatedModules` — the default in Vite, Next.js, SvelteKit, ts-jest, and swc/esbuild —
cannot import any of them: `TS2748: Cannot access ambient const enums when
'isolatedModules' is enabled`. The repository's own `tsconfig.json` omits that flag,
which is why CI stayed green while the published package was unusable under those
toolchains.

## What Changes

- Declare the eleven enums in `index.d.ts` in the same form the build emits, so a
  transpiling TypeScript consumer can import them. Non-breaking: the runtime already
  exposes every one of them as a real string-enum object.
- Declare `SheetState` in `index.d.ts`. It is a live runtime export with no type
  declaration today, so `import { SheetState } from 'excelrs'` fails to compile while
  plain JavaScript works. `WorksheetState` is retained as an alias, preserving the
  ExcelJS-compatible name consumers may already use.
- Teach `scripts/verify-build-output.cjs` to assert that the published declarations and
  the generated declarations agree, so this class of divergence fails the build instead
  of shipping. The check runs in CI and release already; no pipeline change is needed.
- Delete the hand-written re-export block in `index.js`. `module.exports = nativeBinding`
  already exposes the whole binding, so all 22 following assignments are no-ops with no
  renames among them — a second hand-maintained list of the same enum and class names
  that can drift, with nothing checking it.

No breaking changes. The `WorksheetState` name is kept, and no enum value changes.

## Capabilities

### New Capabilities

None. `package-entrypoint` already owns the published type declarations and the
assertions that verify them; this extends it rather than introducing a near-duplicate.

### Modified Capabilities

- `package-entrypoint`: three requirements added, extending the published-types contract
  beyond the `getCell` overloads to cover the declaration form of every exported enum, the
  completeness of the published export surface, and a check that compares the published
  declarations against the generated ones. The existing verification requirement is left
  intact — the new concerns are separate behaviors and are stated as separate
  requirements rather than folded into it.

## Impact

- `index.d.ts` — eleven enum declarations lose the `const` modifier; `SheetState` added;
  `WorksheetState` becomes an alias of it.
- `scripts/verify-build-output.cjs` — gains a published-vs-generated agreement check. It
  already requires the generated file to exist, so no new precondition is introduced.
  Callers are unaffected: `.github/workflows/ci.yml:63`,
  `.github/workflows/release.yml:122`, and `pnpm verify:build`.
- `index.js` — 22 no-op re-export assignments removed. No export changes shape; verified
  that every listed name is already reachable and that none is a rename.
- `biome.json` — unchanged. It already lints `index.d.ts`, so the eleven
  `noConstEnum` warnings go to zero without a config edit.
- Consumers on `isolatedModules` toolchains gain the ability to import the enums.
  Consumers importing `SheetState` gain a working type. No consumer loses anything.
