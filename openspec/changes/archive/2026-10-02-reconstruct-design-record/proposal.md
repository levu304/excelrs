# Proposal

## Why

The project's design-decision record is unaddressable and its spec corpus encodes mutable
inventories as normative requirements, so both rot silently — as demonstrated twice in the
same week by changes `reconcile-parity-spec-drift` (#66) and `split-long-requirements` (#67),
and still visible *inside* the spec #66 just reconciled.

Three concrete conditions motivate this now:

- **Requirements restate machine-readable inventories.** `release-verification` enumerates
  "the seven published packages" by name, and a trailing scenario still says "all **5** npm
  packages" — a count that contradicts the requirement 40 lines above it in the same file.
  #66 corrected the first occurrence and missed the second.
- **Per-platform capabilities are a changelog, not a taxonomy.** `musl-support` and
  `linux-arm64-support` own six requirements to record two platform *increments*. The three
  always-present targets (darwin-arm64, linux-x64-gnu, win32-x64-msvc) have no spec at
  all, and `package.json` declares a sixth target. Adding a target forces another
  increment-shaped capability.
- **The decision record has no index and no durable home.** 27 ADRs exist only as a
  `# | title | one-line rationale` table in `docs/spec.md`; 118 further `### D<n>.`
  decisions sit across 77 archived change directories whose `D1` numbering collides (six
  distinct meanings); exactly one decision was ever extracted to `docs/adr/`. Meanwhile
  `openspec/config.yaml` is a bare template with no project context.

## What Changes

- **Introduce a `design-record` capability** governing the durable decision record: ADR
  files under `docs/adr/` with an index and explicit status, and a reduced `docs/spec.md`
  that carries only what a requirement cannot state (FFI constraints, crate/module
  architecture, rejected alternatives). The normative body of `docs/spec.md` is retired in
  favour of `openspec/specs/*`, which already governs behaviour and is CI-gated.
- **Reconstruct the rotated ADRs from primary sources.** Twelve of the 27 in-table ADRs are
  superseded, factually wrong, or carry a trigger that fired un-actioned. Each is rebuilt
  with status and provenance rather than copied, because `docs/spec.md` retains only
  one-line rationales — the reasoning must be recovered from the two architecture reviews,
  the archived `design.md` corpus, and git history.

  Audited against v2.9.1, the 27 classify as: 15 current; 1 already annotated resolved in
  place (ADR-19); 4 fully superseded by shipped features (8, 17, 24, 25); 2 carrying stale
  inventories (10, 23); 1 factually wrong (22); 1 half-superseded yet still cited as
  current policy (9); 1 whose own re-evaluation trigger fired un-actioned (2); and 2 whose
  standing cannot be settled without reading the current reader (3, 15).
- **Resolve the two ADRs that are open decisions, not rot.** ADR-2 (monolithic crate,
  "re-evaluate at ~5K LoC" at 26,258 LoC) is superseded with the trigger the codebase shape
  actually warrants. ADR-9 ("formula evaluation is a separate product") is superseded to
  record that the product was partly built — `recalculate()` shipped in v2.9.0.
- **Correct the ADR-9 citation in `ROADMAP.md`**, which currently cites that superseded
  decision as governing policy for the `formula-eval` engine.
- **Introduce a `platform-targets` capability** with one invariant — every target in the
  release build matrix is published, resolvable, and smoke-tested — and retire
  `musl-support` and `linux-arm64-support`. The target list lives in `release.yml`; the
  spec points at it rather than restating it.
- **Reshape three `release-verification` requirements** from restated inventories into
  invariants anchored on `release.yml`, including removing the self-referential
  "Release package set matches the release-verification spec" and the stale `5`-package
  scenario.
- **Extend `spec-integrity` with authoring rules** so the class stops recurring: requirements
  state invariants rather than inventories, state one behavior each, and cite only decisions
  that are current.

No code, API, dependency, or published-package change. Docs, specs, and CI only.

## Capabilities

### New Capabilities
- `design-record`: The durable design-decision record — ADR files with an index and status,
  and a `docs/spec.md` reduced to the non-derivable rationale layer, so a decision's
  rationale and current standing are both reachable without archaeology.
- `platform-targets`: Every target in the release build matrix is built, published as a
  resolvable optional dependency, and verified by a smoke test — stated once as an
  invariant anchored on `release.yml` rather than as per-platform inventory.

### Modified Capabilities
- `spec-integrity`: Adds authoring rules that prevent the rot class — requirements state
  invariants rather than restating machine-readable inventories, state one behavior each,
  and cite only currently-valid decisions.
- `release-verification`: The trusted-publishing and package-set requirements stop
  enumerating package names and counts inline and become invariants anchored on
  `release.yml`; the self-referential package-set requirement and the stale `5`-package
  scenario are removed.
- `musl-support`: All three requirements removed; the capability is retired into
  `platform-targets`. Musl-specific knowledge that is not an invariant (dlopen-safe
  linking; fully-static musl segfaults on load) is preserved as a scenario.
- `linux-arm64-support`: All three requirements removed; the capability is retired into
  `platform-targets`.

## Impact

- **Affected documents:** `docs/spec.md` (reduced), `docs/adr/` (index + reconstructed ADRs),
  `ROADMAP.md` (ADR-9 citation), `openspec/config.yaml` (project context and per-artifact
  authoring rules).
- **Affected specs:** two new capabilities, two modified, two retired.
- **Sources for reconstruction:** `docs/architecture-review.md`,
  `docs/architecture-review-pass2.md`, 77 archived `design.md` files under
  `openspec/changes/archive/`, and git history.
- **Explicitly not affected:** no Rust source, no `index.js` / `index.d.ts` / `native.*`, no
  `package.json` napi targets, no `release.yml` matrix, no published package contents. The
  build matrix is read as the source of truth, never modified to match a spec.
