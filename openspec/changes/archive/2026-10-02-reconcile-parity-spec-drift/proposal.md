# Proposal

## Why

`openspec/specs/` is the declared source of truth for what excelrs is, but nothing verifies
it, and it has drifted from the code in ways that are now load-bearing:

- **`exceljs-parity` contains a `SHALL` the shipped code violates.** It requires that
  `sheet state (visible/hidden)`, `tab color`, and `default worksheet properties` "remain
  `planned` / `n-a`" as out of the completed v1.x program. `worksheet-metadata` shipped all
  three on 2026-08-08 (`fd4e84c`) and correctly recorded `Modified Capabilities: none` —
  the spec gave it nothing to hook onto. The spec froze at a v2.0.0 release snapshot and
  has no vocabulary for "triaged and shipped."
- **`release-verification` requires publishing "all 5 npm packages"** and names five. The
  release workflow publishes **7** (`publish-musl-bindings` added musl x2) and its own
  verify step is titled `Verify all 7 packages on npm`.
- **`musl-support/spec.md` is a delta spec archived as a main spec** — it carries
  `## ADDED / ## REMOVED / ## MODIFIED Requirements` instead of `## Requirements`. It fails
  `openspec validate`, reports **0 requirements** to `openspec list --specs`, and its three
  shipped requirements are invisible to any tooling that reads the inventory.
- **`ROADMAP.md`'s parity matrix still marks those same three rows `planned` / "Not
  implemented"**, and `docs/spec.md`'s header still reads `Version: 1.0.0` / `Next: v2.0.0`.
- **`ci.yml` never runs `openspec validate`.** No spec is ever checked against the spec
  schema, so all of the above accumulated silently across ~9 minor releases.

## What Changes

- **Add a spec-integrity gate to CI** — `openspec validate --strict --specs --changes`
  runs on every push and PR to `main`, failing the build on ERROR-level findings.
- **Fix `musl-support/spec.md` structurally** — convert the delta headers to a canonical
  `## Requirements` section, preserving all three requirements verbatim (no requirement
  text changes; this is a structural repair that restores them to the inventory).
- **Retarget the `exceljs-parity` exclusion requirement** so it records the v1.x program's
  completion as *historical* fact and no longer asserts a permanent `planned` state for
  areas that have since shipped. Shipped items move out of the exclusion list; the durable
  invariant ("every area carries exactly one status; the matrix records the program
  complete") is preserved.
- **Correct `release-verification`'s package count** from 5 to 7 and name all seven,
  matching `release.yml`.
- **Reconcile the `ROADMAP.md` parity matrix rows** that the code has overtaken, and refresh
  the stale version headers in `ROADMAP.md` and `docs/spec.md` so they no longer claim to
  describe v1.x.
- **Write real `## Purpose` sections for the 18 specs** still carrying the
  `TBD - created by archiving change ...` placeholder, so `openspec validate --strict`
  is warning-clean.

Non-goals: this change does **not** build a matrix generator, does not touch
`streaming-write-incremental`'s presence in `specs/` (a separate category question), and
does not prune stale git branches or reconcile the `Cargo.toml` / `package.json` version
split.

## Capabilities

### New Capabilities

- `spec-integrity`: The project's OpenSpec corpus stays mechanically valid and
  non-contradictory — specs are validated in CI, every capability is discoverable through
  the standard inventory, and no spec carries a placeholder `## Purpose`. This capability
  owns the *health* of the spec corpus, not the behavior of any shipped excelrs feature.

### Modified Capabilities

- `exceljs-parity`: The "parity program complete" requirement no longer asserts that
  sheet state, tab color, and default worksheet properties SHALL remain `planned` /
  `n-a`. It is rewritten to record the v1.x completion as historical fact plus a durable
  status invariant that survives later triage and shipping.
- `release-verification`: The npm trusted-publishing requirement updates from five
  published packages to seven, naming the two musl platform packages, so the spec matches
  `release.yml`.

## Impact

- **Specs (modified in place)**: `openspec/specs/exceljs-parity/spec.md`,
  `openspec/specs/release-verification/spec.md`, `openspec/specs/musl-support/spec.md`,
  and the 18 specs with placeholder `## Purpose` sections.
- **CI**: `.github/workflows/ci.yml` gains an `openspec validate --strict` step. Requires
  the OpenSpec CLI to be available in CI (new setup step or `npx` invocation) — a first-time
  CI dependency for this repo.
- **Docs**: `ROADMAP.md` (parity matrix rows + header), `docs/spec.md` (version header).
- **Not affected**: no Rust source, no `index.js` / `index.d.ts` / `native.*`, no runtime
  behavior, no published package contents. This is a spec-and-docs correctness change plus
  a build gate.
- **Risk**: the new gate is expected to fail on first run (that is the point). The bulk of
  the work is landing before the gate is enabled, or enabling it with the fixes in the same
  commit — see `design.md`.
