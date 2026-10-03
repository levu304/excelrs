# Design

## Context

See proposal.md — Why for the motivation. What shapes the approach:

- `napi build` appears in five places: two scripts in `package.json` (`build`,
  `build:release`) and three workflow steps (`.github/workflows/ci.yml`, and two in
  `.github/workflows/release.yml` — musl cross-compile and non-musl emnapi).
- `scripts/verify-feature-parity.cjs` already discovers all five, anchors the
  comparison on `package.json`, and ships a probe suite at
  `__test__/verify-feature-parity.test.ts` that mutates sandboxed copies of `package.json`
  and `.github/` to prove the checker fails and passes when it should.
- That checker parses invocation text with regexes rather than a YAML parser, by a
  deliberate trade recorded in its own `ponytail:` ceiling comment. Each parsing hole it
  has ever had was found by mutating a scratch copy of `.github/`, not by reading the
  code.
- `native.d.ts` is a build artifact consumed by `tsc`. The package publishes the
  hand-maintained `index.d.ts`. Nothing a consumer imports is affected by these flags.

## Goals / Non-Goals

**Goals:**

- Make every `napi build` invocation declare the same type-declaration flags, anchored
  on `package.json`.
- Make a future divergence of those flags fail CI, in the same run that already checks
  Cargo feature parity.
- Keep the addition small enough that it does not become a second place to update.

**Non-Goals:**

- Refactoring the build flags into a single shared definition. Rejected below; the
  release matrix passes per-leg flags the scripts do not carry.
- Changing what the flags mean, or the published declarations they produce.
- Re-parsing workflows with a real YAML library.

## Decisions

### Fix the invocations, not the gate that caught them

`verify-build-output.cjs` reports a const divergence it is correct to report.

**Alternative considered:** relax the enum `const`-ness comparison so a `const enum`
generated file passes.

**Rejected.** That re-admits the exact `TS2748` regression the parent change exists to
close, and there is no consumer reason to accept the form: `native.d.ts` is unpublished.
The gate is the only thing in the repo that reads enum declaration form, which is
precisely why it found a divergence that had survived every release. Weakening it
removes the sole detector.

### Anchor the comparison on `package.json`, not on flag names in code

The new comparison reuses the checker's existing shape: a reference invocation
(`package.json`'s `build`, already `invocations[0]`) and a per-invocation `missing` /
`extra` diff. It compares declared flag names; it does not hardcode which flags exist.

**Alternative considered:** require `--no-const-enum` and `--runtime-string-enum` by
name in the checker.

**Rejected.** That restates a value that already lives in `package.json`, which is the
failure mode the project's own specs rule calls out — a restated inventory goes stale
the next time the source changes and no validator can catch it, because it stays
structurally valid while being semantically false. Naming the source and requiring
agreement survives an napi flag rename without a code change.

### Parity alone is not sufficient, and that is fine

If the flags were stripped from `package.json` as well, every invocation would agree
and the parity check would pass.

This is deliberate and covered, not a gap. `verify-build-output.cjs` compares the
published declarations against the generated ones and fails on a `const` divergence, so
an all-stripped state fails that check instead. The two requirements compose: parity
catches a drifting invocation, and declaration-form agreement catches a changed intent.

**Consequence:** the parity checker needs no allowlist, no "expected minimum" set, and no
degenerate-case guard of its own for this comparison. It stays a diff.

### Extend the existing checker instead of adding a script

`verify-feature-parity.cjs` already discovers the five invocations, already flattens
folded and `\`-continued commands, and already has a probe suite with
`expectRejected` / `expectAccepted` helpers.

**Alternative considered:** a new `verify-build-flag-parity.cjs`.

**Rejected.** It would re-implement invocation discovery and re-implement the
degenerate-case handling, and add a third CI step and release step to wire. The
discovery is the fragile part; duplicating it doubles the surface the `ponytail:`
ceiling comment warns about.

**Accepted trade-off:** the script's name says feature parity but it now covers
type-declaration flags too. Renaming it would touch CI wiring, the release workflow, and
the existing probe suite for no behavioral gain. Not worth it now; if a third flag family
ever needs checking, rename and widen at once.

### Reuse the flatten step rather than re-deriving it

The existing `parseFeatures` flattens YAML block scalars and `\`-continuations before
matching. The new comparison reads from the same flattened command text, so a musl step
that continues onto the next line is read the same way the feature set already is.

**Do not** write a second flattening helper. Two implementations of the same fragile
step is the defect, not the fix.

## Risks / Trade-offs

- **Regex parsing could read a new invocation shape as "declares nothing"** → extend the
  existing probe suite with the cases for this comparison, including a cross-compile step
  carrying `--target` and `--cross-compile`, which are not type-declaration flags and must
  be ignored. Re-run the pre-existing probes afterwards; the file's own ceiling comment
  requires it after any parsing change.
- **An all-agree state passes parity** → covered by the declaration-form comparison in
  `verify-build-output.cjs`, by composition rather than by a second guard.
- **Merge ordering: this must land before PR #71, or PR #71 stays red** → sequence the
  merges deliberately. PR #71 has been red since its first implementation commit; this
  is the fix for that, not a consequence of it.
- **Release legs are unverifiable locally** → the musl and emnapi build steps only run in
  the release matrix. Treat a green release dry-run, or at minimum the non-musl leg, as
  the confirming signal rather than assuming local parity implies release parity.

## Migration Plan

None. No data migration, no published type change, no runtime change. The correction
takes effect on the next CI and release run.