# Proposal

## Why

Three of the five `napi build` invocations in this repo omit `--no-const-enum` and
`--runtime-string-enum`, so CI and release generate a `native.d.ts` that declares
`const enum` while a local `pnpm build` emits a plain `enum`. Nothing compared the two,
so the divergence survived every release. It surfaced the moment
`verify-build-output.cjs` began reading enum declaration form: all three CI legs fail,
and the release workflow's verify step would fail with them.

The build is only correct where it happens to be spelled correctly. The flags are
duplicated across five invocations and three of the copies are wrong.

## What Changes

- Add `--no-const-enum --runtime-string-enum` to the three workflow invocations that
  lack them: the CI build step and both release build steps (musl cross-compile and
  non-musl emnapi). `package.json`'s `build` and `build:release` already carry them.
- Extend `scripts/verify-feature-parity.cjs` to compare type-declaration flags across
  every `napi build` invocation, not only `--features`. The checker already enumerates
  all five invocations and already anchors on `package.json`; it simply never looked at
  these flags, which is why it passed over a three-way divergence.
- Extend the checker's probe suite with the cases that would have caught this drift.

Not breaking. The generated `native.d.ts` is a build artifact consumed by `tsc`; the
package publishes the hand-maintained `index.d.ts` as its `types` entry. Correcting the
flags changes no published type, no exported name, and no runtime value — napi emits
the enum objects regardless of how the declaration is written.

## Capabilities

### New Capabilities

None. This is a defect in how existing behavior is produced, not behavior the system
does not yet have.

### Modified Capabilities

- `package-entrypoint`: adds a requirement that every `napi build` invocation declares
  the same type-declaration flags as the `package.json` build scripts. This capability
  already owns the requirement that published and generated declarations agree on enum
  `const`-ness; this is the upstream cause of that requirement failing, and the two
  together close the loop — flag drift is caught at the invocation, and a change to the
  declared flags themselves is caught by the existing declaration-form comparison.

## Impact

- `.github/workflows/ci.yml` — one build step gains two flags.
- `.github/workflows/release.yml` — two build steps gain two flags. Without this the
  release fails at "Verify build output carries the glue transforms".
- `scripts/verify-feature-parity.cjs` — one additional comparison, reusing the existing
  invocation discovery and the existing `package.json`-anchored reference.
- `__test__/verify-feature-parity.test.ts` — probe cases for the new comparison.
- Unblocks CI on `fix/align-published-type-declarations` (PR #71), which has been red
  since its first implementation commit.
- No consumer-facing change: `native.d.ts` is unpublished, and the published surface is
  untouched.
- Ordering note: this must land before PR #71 merges, or PR #71 stays red. It is a
  separate change from PR #71 on purpose — one OpenSpec change per PR keeps the archive
  and the changelog honest.