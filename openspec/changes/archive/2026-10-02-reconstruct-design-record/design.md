# Design

## Context

See `proposal.md` — Why. The facts that shape the approach:

- `docs/spec.md` is 1400 lines. Its 27-row ADR table holds only `# | title | one-line
  rationale`; the body prose cites ADRs but never explains one. The reasoning behind those
  decisions is therefore **not present in the file** and must be recovered from elsewhere.
- Three surviving sources hold that reasoning: `docs/architecture-review.md` (27.9K) and
  `docs/architecture-review-pass2.md` (22.2K), both pre-implementation reviews of spec
  v1.0.0/v1.1.0 that resolved named P0/P1 issues; 77 archived change directories under
  `openspec/changes/archive/` holding 118 `### D<n>.` decisions; and git history.
- An audit of all 27 in-table ADRs against v2.9.1 classifies them as: 15 current, 8 fully
  superseded, 2 carrying stale inventories, 2 factually wrong, 1 half-superseded yet cited
  as current policy (`ROADMAP.md` → ADR-9), and 1 whose own re-evaluation trigger fired
  un-actioned (ADR-2, "re-evaluate at ~5K LoC" at 26,258 LoC).
- `docs/adr/` contains exactly one file, `005-streaming-write-buffering.md`. Its number
  **collides** with row 5 of the spec.md table (dual MIT/Apache-2.0 license) — the two
  numbering streams are unrelated.
- The `release.yml` matrix declares 6 targets; `package.json` `napi.targets` declares the
  same 6; the workflow verifies "all 7 packages" (6 platform + 1 main). The three
  always-present targets have no capability, and the two most recently added ones each own
  a capability.
- `openspec/config.yaml` is a bare template. `openspec validate --strict --specs --changes`
  is already a CI gate (`ci.yml`, introduced in #66).

## Goals / Non-Goals

**Goals:**

- Make every design decision's standing and reasoning reachable from an index, without
  git archaeology.
- Leave the corpus with fewer places that can contradict the code than it has today.
- Land authoring rules in `openspec/config.yaml` so the rot class is not reintroduced by
  the next change.
- Keep the whole change reversible: docs, specs, and CI only.

**Non-Goals:**

- Recovering the 118 archived `D<n>.` decisions into `docs/adr/`. They are addressed by
  the archive path they already live at; transcribing 77 directories is a separate, much
  larger job with a different justification.
- Rewriting the normative body of `docs/spec.md` into a v3 document. The body dies; see D1.
- Building a matrix generator, or any validator for citation drift. See D6.
- Reconciling the pre-existing, unrelated drift noted in #66's own non-goals: the 20 stale
  branches, and `Cargo.toml` version `0.14.0` vs `package.json` `2.9.1`.
- Changing the release matrix, `package.json` targets, or any published package. Those are
  read as the source of truth, never edited to match a spec.

## Decisions

### D1: `docs/spec.md` survives as the non-derivable layer, and its body is deleted

The reduced file keeps only what a requirement cannot state: the FFI constraints (why
`CellValue` is a flat `#[napi(object)]` struct, why a JS glue layer exists, why `cellXfs`
dedup uses `BTreeMap`), the crate/module architecture, and the rejected alternatives. It
opens by pointing to `openspec/specs/` for behavior.

*Why this and not a full rewrite to v3:*

- *Full rewrite* — refreshing §1–§12 (API design, module breakdown, data flow) means
  restating behavior the specs already own and CI already gates. That rebuilds the
  duplication which caused #66, in a document with no gate at all. A second normative
  surface is strictly more rot surface.
- *Retire the file entirely* — discards the FFI and architecture rationale, which is
  unrecoverable from specs and currently stranded in a 64K file whose header admits it is
  not authoritative. The reviews that would replace it are about spec v1.0.0, not v2.9.1.
- *Reduce* — keeps the non-derivable knowledge, deletes the rest. A document that states no
  observable behavior has nothing to contradict.

**Consequence to accept:** the surviving file is short — plausibly 150–300 lines. That is the
intended outcome, not a shortfall. Its value is measured by whether a reader can answer
"why is it shaped like this", not by completeness.

### D2: The spec.md table's 1–27 numbering becomes canonical; the stray `005` is renumbered

Each surviving decision is written to `docs/adr/NNN-<slug>.md` using its row number from the
spec.md table. `005-streaming-write-buffering.md` does not correspond to table row 5 and is
renumbered to the next free slot (28) rather than left to collide.

*Why not renumber everything to a new scheme:* the 1–27 numbers are already cited in spec.md
body prose and in `CHANGELOG.md`. Reusing them keeps those citations resolvable, which is
the entire point of the exercise.

*Why not merge the streaming decision into an existing row:* it is a distinct decision
(buffering strategy for incremental streaming writes, added in v2.0.0) with no table row.
Giving it a real number is cheaper than forcing it into an unrelated one.

### D3: Reconstruction states what it recovered and marks what it could not

A recovered decision file is written from the strongest available source, in this order:
the change's own archived `design.md` (contemporaneous, authored at decision time); the
architecture reviews (authored against the spec that encoded the decision); git history and
the commit that introduced the feature; and only then the one-line rationale in the spec.md
table.

Each reconstructed file names its sources. **Where a decision's rationale cannot be
recovered, the file records the decision and its status and states plainly that the
rationale is unrecovered.** It is not inferred, and plausible-sounding reconstruction is
not written.

*Why this and not reconstructing in full:* a decision record whose stated reasons were
invented after the fact is worse than one that admits a gap — it will be trusted, and it is
wrong. The `design-record` capability's provenance requirement exists to make the difference
visible.

*Cost:* some files will be thinner than others. That is the honest result.

### D4: Two statuses only — `current` and `superseded`

*Why not a third status for "decided but not yet implemented":* no such decision exists in
the project today, and #66's D1 explicitly rejected inventing a status that "only ever
applies once" as "more machinery than the problem needs." Adding one now repeats that
mistake in the artifact meant to prevent it. If a proposed decision ever needs recording, the
vocabulary can be added then.

### D5: Rotated ADRs are resolved individually, not blanket-superseded

Each of the 12 non-current ADRs is resolved on its own merits:

| ADR | Disposition |
|---|---|
| 8, 17, 24, 25 | `superseded` by the features that shipped (styles CRUD, rich text, hyperlink, theme-color write) |
| 10 | `superseded` — its target list is now the `release.yml` matrix |
| 23 | `superseded` — the ~440 KB figure is stale by ~40x (actual darwin-arm64 addon: 17.3 MB); restated as measured, or dropped if the figure is no longer a useful constraint |
| 22 | Corrected, not superseded — it says `napi-build = "3"` while `Cargo.toml` has `"2"`; the ADR's own parenthetical already noted both work, so the intent stands and the number is wrong |
| 9 | `superseded` — the "separate product" was partly built: `recalculate()` shipped in v2.9.0 and `Cargo.toml` carries `xlstream-parse`/`xlstream-core` behind `formula-eval` |
| 2 | `superseded` with the replacement trigger stated — see below |
| 3, 15 | Verify against the current reader before dispositioning; both concern calamine's role, which the SAX streaming path may have changed |

**ADR-2 specifically.** The decision ("monolithic crate; re-evaluate at ~5K LoC") fired its
own trigger around 5K LoC and was not actioned; the crate is 26,258 LoC across 32 files.
Inspection shows the crate-split lever no longer fits: the module boundaries a split would
create already exist (`model/` 20 files, `reader/` 4, `writer/` 3, `formula/` 3, `xlsx/` 2),
and the concentration that actually matters is file-level — `src/model/worksheet.rs` is
1,933 lines against an ~820-line average. The superseding decision therefore replaces the
size trigger with the one the current shape warrants, and records that the original trigger
was well-calibrated for a different codebase shape. This is a finding, not a refactor: no
code changes here.

*Why not simply mark ADR-2 `current` and move on:* its stated re-evaluation condition is
objectively met. Leaving it unmarked preserves a decision that instructs the reader to do
something nobody did.

### D6: Citation drift is not gated; it is prevented at the point of supersession

When ADR-9 is marked `superseded`, `ROADMAP.md` is corrected in the same change. That is
the whole mitigation.

*Why no automated check:* a validator would have to resolve markdown citations to ADR files
and compare status — plausible, but it would run on prose that is, by D1, being actively
deleted, and #66 already established the project's position that semantic rot is handled by
"human discipline, backed by review, not automation." Adding a second CI step here to police
a file that is about to shrink is not worth the maintenance. The structural gate already
catches structural rot; this class is a review responsibility and the `design-record`
capability states it as a requirement so it is at least written down.

### D7: `config.yaml` gets context and authoring rules, split by what they are for

`context` records what an agent needs before touching this repo and cannot infer: the
stack (Rust + napi-rs v3, `zip` + `quick-xml` for write, calamine for read, SAX for
streaming), the authority order among `openspec/specs/`, `docs/spec.md`, `ROADMAP.md`, and
`CHANGELOG.md`, and the fact that `release.yml` and `package.json` are the source of truth
for the build matrix. `rules` carries the per-artifact authoring rules surfaced by this
change: state one behavior per requirement, do not restate a machine-readable inventory,
and keep requirement prose under the validator's length threshold.

*Why the split matters:* `rules` is keyed by artifact id and applies only when writing that
artifact, which is exactly the scope of an authoring convention. Putting conventions in
`context` would apply them to every task including non-authoring ones.

### D8: Retiring a capability is done by removing all its requirements

`musl-support` and `linux-arm64-support` are retired by REMOVING every requirement, not by
deleting the spec directories directly. Each removal carries a Reason and a Migration per
the delta format, and each names the `platform-targets` requirement that takes over.

*Why remove rather than re-point:* the six requirements have no unique content left after
`platform-targets` exists except the musl dynamic-linking constraint, which is preserved as
a scenario there. Retaining rewritten shells would recreate the incremental-per-platform
pattern this change exists to end.

**Verification before archiving:** diff the union of removed requirements against
`platform-targets` requirement by requirement, and confirm no obligation is dropped. The
musl dynamic-linking constraint and the musl smoke-test obligation are the two most likely
to be lost and are called out explicitly in the deltas.

## Risks / Trade-offs

**[Reconstruction invents rationale that did not exist]** → D3 makes unrecovered rationale
explicit rather than inferred, and every file names its provenance. A thin, honest file beats
a confident wrong one.

**[A retired capability's directory survives archive as an empty or placeholder spec]** →
Check `openspec list --specs` after archiving; if a capability still appears with zero
requirements, remove the directory. The `spec-integrity` requirement on non-placeholder
Purpose sections means an empty spec would surface in `--strict` validation.

**[Removing the platform capabilities loses operational knowledge]** → D8's verification
step, plus explicit migration notes naming the replacement requirement for each of the six.

**[The reduced `docs/spec.md` is read as an omission]** → Its opening section states what was
removed and where it now lives, and `openspec/config.yaml` context carries the same
authority order. A short document that says what it is for is better than a long one nobody
trusts.

**[`openspec/config.yaml` context goes stale]** → It describes the stack and the authority
order, both of which change far less often than specs do. Accepted residual risk; it is
documentation, and a wrong line is cheaper to notice than a wrong requirement.

**[ADR-3 and ADR-15 cannot be dispositioned without reading the current reader]** → Disposition
them from verified source in `src/reader/` and `src/stream.rs`; if the calamine role turns out
to be genuinely ambiguous, record the ambiguity in the ADR rather than asserting a resolution.
This is the one part of D5 that is investigation rather than a known answer.

**[Fixing ADR-9's citation reopens the product-thesis question]** → This change corrects a
factual misstatement only. Whether `formula-eval` should be promoted out of its opt-in
feature is a separate decision and explicitly not made here; if the correction surfaces it,
it becomes its own change rather than expanding this one.

## Migration Plan

Documentation and specification only. No code, no API, no dependency, no published package
changes, so rollback is `git revert` of the merge commit with no consumer impact. The
platform-spec retirement is the only change that removes specification text; the verification
step in D8 runs before archive, and the pre-archive state is recoverable from git.

## Open Questions

- Whether `openspec archive` removes a capability directory whose requirements are all
  removed, or leaves an empty spec behind. Deferrable: it affects a cleanup step after
  archive, not the specs, the approach, or the task breakdown.
- Whether the `exceljs-parity` capability's matrix rows need a corresponding platform-area
  update now that two platform capabilities are retired. Deferrable to a follow-up that
  already has an owner in the parity-matrix requirement; it does not block this change.
