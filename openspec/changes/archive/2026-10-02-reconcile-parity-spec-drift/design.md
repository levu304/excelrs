# Design

## Context

See `proposal.md` — Why. The design-relevant facts:

- `ci.yml` has **no OpenSpec step and no OpenSpec dependency**. The CLI is not currently
  available in CI; it must be introduced.
- `openspec validate --specs` currently reports, across 40 specs: **1 INVALID**
  (`musl-support`), **18 WARNING** (placeholder `## Purpose`), **12 INFO** (long
  requirement text). Only the INVALID is an error; the WARNINGs only surface under
  `--strict`.
- Two requirement bodies in `exceljs-parity` and `release-verification` are **semantically
  obsolete but structurally valid**. No validator run — strict or not — can catch these.
  They are wrong relative to the *code*, not relative to the spec schema.
- `musl-support`'s three requirements are correct as written; only the section headers are
  wrong. The repair is mechanical.

**An open question carried in from exploration, resolved here:** was `worksheet-metadata`
leaving `exceljs-parity` untouched deliberate (the exclusion list is historical record,
the matrix rows are the live truth) or an oversight (the spec needs a "triaged and
shipped" state)? The design picks the option that is correct under *both* readings — see
D1.

## Goals / Non-Goals

**Goals:**

- Make spec validation a build gate, so this class of rot cannot silently accumulate again.
- Restore `musl-support`'s three requirements to the spec inventory.
- Remove the two obsolete `SHALL`s so no spec asserts something the code contradicts.
- Bring the `ROADMAP.md` matrix and the stale version headers back in line with the code.
- Get `openspec validate --strict` clean, so the gate can be strict from day one.

**Non-Goals:**

- Building a matrix generator. The prioritization half of the roadmap (compat value vs
  effort) is irreducibly human judgment; a generator that cannot rank features is just a
  fancier stale table. Hand-maintained, with the gate and the new matrix-row requirement
  supplying the discipline it has been missing.
- Deciding whether `streaming-write-incremental` belongs in `specs/` at all. That is a
  separate category question about what the directory means, and it does not block any
  task here.
- Pruning the 20 stale branches, reconciling `Cargo.toml` (0.14.0) vs `package.json`
  (2.9.1), or touching any Rust source.

## Decisions

### D1: Record the v1.x program completion in past tense, and add a separate live requirement

The `exceljs-parity` "parity program complete" requirement is rewritten from a standing
constraint into a historical record, and a **new** ADDED requirement ("Parity matrix rows
reflect shipped behavior") carries the live invariant.

*Why this and not the alternatives:*

- *Delete the exclusion list entirely* — loses the v2.0.0 record of what was deliberately
  out of scope, which is genuinely useful context for anyone reading the parity history.
- *Keep the requirement and just remove the three shipped areas from the list* — still
  leaves a requirement whose subject ("when v2.0.0 was cut") is frozen, so the next
  triage cycle hits the identical trap. It also leaves no requirement that would have
  caught this drift.
- *Add a "triaged and shipped" state to the spec* — invents vocabulary for a status that
  only ever applies once. More machinery than the problem needs.

Past-tense + a separate live requirement is correct whether the original omission was
deliberate or accidental: if deliberate, the historical record is now explicit; if
accidental, the new requirement is the hook the next change needs. It also means the
`worksheet-metadata` proposal's `Modified Capabilities: none` was an honest reading of the
spec as written — the spec was underspecified, not the author careless.

### D2: Enable the gate in the same commit as the fixes, not before

The gate is expected to fail on first run. Landing it before the corpus is clean means a
red `main` (or a gate added with `|| true`, which is worse — a gate that cannot fail).
So: **fix first, gate last**, in the same PR. Task ordering enforces this; §1 of tasks.md
gates §6.

*Why not a warning-only phase-in:* `--strict` WARNINGs (the 18 placeholder Purposes) are
exactly the signal we want. Splitting "structural gate" from "strict gate" across two PRs
doubles the ceremony for no benefit, since the corpus fix is mechanical and bounded.

### D3: `openspec validate --strict --specs --changes`, non-interactive, fail on ERROR

Covers both the main specs and in-flight changes, so a malformed delta is caught at PR
time rather than at archive time — which is how `musl-support` got into the corpus in the
first place.

*Why `--changes` and not just `--specs`:* the archive step is where delta headers leak
into a main spec. Validating changes pre-archive is the cheapest interception point.

*Why non-interactive:* CI has no TTY; an interactive prompt would hang the job. The
existing `--json` / `--no-interactive` flags make the outcome deterministic.

*What this gate does **not** do:* catch a structurally valid requirement that the code
contradicts. That is the D1 fix and the new matrix-row requirement — human discipline,
backed by review, not automation. Recorded as a known limit rather than papered over.

### D4: CI invokes the CLI via `npx @fission-ai/openspec`, version-pinned

The CLI publishes to npm as `@fission-ai/openspec` (currently 1.14.0); the local install
is Homebrew, which CI does not have. Pin the exact version rather than using `@latest`:
an unpinned spec-schema change would break `main` without any commit touching it.

*Alternative rejected:* adding it to `devDependencies`. That couples spec tooling to
`pnpm install --frozen-lockfile`, so a lockfile refresh becomes a gate on `main` for a
tool that has nothing to do with the shipped package. `npx` in the workflow keeps the blast
radius to the one step.

### D5: `musl-support` repair preserves requirement text verbatim

Only the section headers change: `## ADDED / ## REMOVED / ## MODIFIED Requirements`
collapse into a single `## Requirements`, and the `## REMOVED (none)` / `## MODIFIED
(none)` blocks are dropped as meaningless in a main spec. No requirement name or text is
touched — the spec-integrity delta states this as a requirement so a future "cleanup"
can't quietly reword a shipped contract.

*Why not re-archive the change:* the underlying change (`2026-08-06-publish-musl-bindings`)
is correctly archived; only the sync into the main spec went wrong. Re-archiving would
rewrite history to fix a formatting bug in one output file.

### D6: Docs reconciliation is a mechanical status update, not a rewrite

`ROADMAP.md` gets the three overtaken matrix rows corrected plus a refreshed header;
`docs/spec.md` gets its version header corrected. `docs/spec.md` is a 64K design document
whose body describes the v1.0.0 architecture — it is **not** comprehensively brought
current in this change, because that is unbounded work with no gate behind it. The header
is corrected so it stops making a false claim about what version it describes, and the
body's staleness is left for a separate, scoped effort.

## Risks / Trade-offs

- **The gate is stricter than the corpus deserves on day one** → land fixes and gate in
  the same commit (D2). `tasks.md` orders §6 last and makes it conditional on §1-§5.
- **`--strict` also flags 12 INFO-level long requirements** → INFO does not fail the
  build; only ERROR does. Those 12 are left alone; shortening them would be requirement
  rewording, which is out of scope and risks losing detail.
- **Pinned CLI version ages** → intentional. An unpinned validator is a supply-chain and
  breakage risk on `main`. Bumping the pin is its own small change.
- **The new matrix-row requirement is unenforceable by tooling** → accepted. It makes the
  expectation explicit at review time and gives the next change a capability to modify.
  A generator would enforce it, at the cost of D-non-goal above.
- **`docs/spec.md` body remains stale after this change** → deliberate (D6), and the
  corrected header no longer asserts it is current. Tracked as a known follow-up, not
  silently absorbed.
- **Risk of scope creep into thread-1 (`streaming-write-incremental`) and housekeeping
  (branches, version split)** → explicitly excluded in the proposal's non-goals. If those
  come up during apply, they are separate changes.

## Migration Plan

Single PR, ordered internally:

1. Repair `musl-support` + correct the two obsolete `SHALL`s + write the 18 Purpose
   sections + reconcile `ROADMAP.md` / `docs/spec.md`.
2. Re-run `openspec validate --strict --specs` locally until clean.
3. Add the CI step last.

Rollback is a single revert; no runtime, published-artifact, or consumer impact at any
point. Nothing here touches the shipped package.

## Open Questions

- Should `openspec validate` also run on `openspec/changes/archive/**` (`--archived`, which
  checks archived changes have all tasks complete)? Out of scope here — worth a separate
  look, since archived changes are historical and a failure there would be noise.
- Should the `INFO`-level long-requirement findings (12 specs) be addressed? Each fix is
  a requirement rewrite, so it is deliberately not bundled with a correctness change.
