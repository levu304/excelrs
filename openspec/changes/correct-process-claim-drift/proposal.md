# Proposal

## Why

Several requirements assert a property of a machine-readable source (`.github/workflows/release.yml`,
`package.json`, `docs/adr/`) while naming no check that compares the two. The claims have drifted
away from the artifacts they describe, and they drifted in the direction that makes each guarantee
sound *stronger* than it is — the worst direction for a contract to fail in.

Two independently-verified examples:

- `release-verification`'s Purpose says its smoke tests "fail the release **before publish**", and
  `platform-targets` requires each matrix target to be smoke-tested "**before the release is
  published**". In `release.yml` the Functional smoke test is step 391 of the `publish` job; the
  platform and main packages are published at steps 300 and 323. A style-loss regression is
  therefore published to npm and *then* fails the release.
- `release-verification` requires the smoke test to stream "a large workbook (one whose row count
  exceeds practical in-memory bounds)". The functional smoke test never calls `wb.stream.*`, and the
  script that does (`scripts/streaming-smoke.cjs`) exercises **one sheet, one row, four cells**.

The same class recurs for `formula-eval`'s unenforced build-feature invariant and for two ADR
citations that resolve to a file which does not contain the cited decision. Each was found by
reading, not by tooling: `openspec validate --strict` reports the corpus as 42/42 passing.

## What Changes

- Correct the release-ordering and scale claims in `release-verification` so they describe what
  `release.yml` actually does, and add the pre-publish check that makes "before publish" true.
- Correct `platform-targets`' functional-smoke-test requirement, which currently attributes a
  per-target pre-publish check to a step that runs once, post-publish, on a single platform.
- Add a `spec-integrity` requirement for the class itself: a requirement asserting an agreement
  between a specification and a machine-readable source names the check that verifies it, or is
  scoped so it cannot become untrue.
- Extend the `design-record` citation rule, which today verifies that a cited decision's *status*
  is `current` but never that the cited file contains the cited decision.
- Give `formula-eval`'s local-build/pipeline feature-set requirement a check, replacing the
  unenforced invariant with a verified one.
- Fix the two `ADR-005` citations in `streaming-write-incremental`, which resolve to the
  dual-license ADR rather than the streaming-buffering decision (`ADR-028`).
- Bring `streaming-smoke.cjs` up to the scale the specification claims, and run the streaming
  round-trip on musl targets where it is currently skipped.

**BREAKING** (behavioral, for consumers only in the sense that a guarantee is being narrowed to
what is true): the release pipeline's pre-publish verification boundary changes. Packages will no
longer be published before the assertions in `release-verification` have run. Existing published
versions are unaffected.

## Capabilities

### New Capabilities

None. `spec-integrity` already owns "the corpus is trustworthy and does not drift silently", and
the project rule is to extend an existing capability rather than introduce a near-duplicate.

### Modified Capabilities

- `release-verification`: the three smoke-test requirements and the Purpose claim a pre-publish
  boundary and a streaming scale that `release.yml` does not implement.
- `platform-targets`: "Every build-matrix target runs the functional smoke test" attributes a
  per-target pre-publish check to a step that runs once, after publication, on one platform.
- `spec-integrity`: gains a requirement covering agreement claims against machine-readable sources
  that name no verifying check.
- `design-record`: "A document citing a decision cites a standing one" verifies standing but not
  that the cited file contains the cited decision.
- `formula-eval`: "The local build matches the pipeline's feature set" is true but nothing compares
  `package.json` against the workflow, and it has already failed once (build script omitted the flag).
- `streaming-xlsx` / `streaming-write-incremental`: the two `ADR-005` citations point at the
  license ADR; the streaming-buffering decision is `ADR-028`.

## Impact

**Workflow:** `.github/workflows/release.yml` — the pre-publish verification boundary moves ahead
of the two publish steps. The Functional smoke test's assertions are unchanged; only their position
in the job changes. Per-target verification continues to live in the `build` job.

**Scripts:** a new verification script comparing feature sets across `package.json` and the
workflows; `scripts/streaming-smoke.cjs` scaled to a workbook that meaningfully exceeds in-memory
bounds.

**Specs:** deltas to six capabilities. No runtime API changes. No Rust source changes. No change to
the `formula-eval` feature's actual behavior — the feature gate and published artifacts are already
correct and are explicitly out of scope.

**Docs:** none required. `ADR-005`/`ADR-028` and `ADR-028`'s own numbering note already record the
renumbering correctly; only the specs that cite the old number need correcting.