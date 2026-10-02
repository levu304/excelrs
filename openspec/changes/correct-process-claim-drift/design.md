# Design

## Context

See `proposal.md` — Why for motivation. The structural facts that shape the approach:

The `publish` job's Functional smoke test (renamed by this change to `Verify the published package
set resolves from npm`) resolves its subject by
`npm install --no-save "@levu304/excelrs@$VERSION"` from the public registry. That is why it ran
after both publish steps rather than before them — it is not mis-ordered, it is *dependent*. The
step order that makes the spec claim false cannot be fixed by moving lines; the check has to
be split into a pre-publish part and a post-publish part. (Steps are named here rather than
cited by line number, because line numbers drift with every unrelated edit to the workflow.)

The `build` job already downloads nothing, but the `publish` job already has every built
binary on disk from `Download platform artifacts`, *before* either publish step.
A pre-publish assertion has the artifacts available; it lacks only the installed-package
resolution that the current step uses.

`spec-integrity` already contains two adjacent rules — "A requirement naming an enforcement
mechanism names a resolvable target" and "An unenforceable contract is not stated as a
standing invariant". The new rules in this change extend that line of reasoning rather than
opening a new one. Notably, `config.yaml`'s design rule says to *prefer deleting a place that
can rot over adding a mechanism to check it*, which bounds how much machinery is worth adding.

## Goals / Non-Goals

**Goals:**

- Make every release claim in `release-verification` and `platform-targets` true as written.
- Preserve the post-publish npm-resolution check, which catches packaging failures (wrong
  `optionalDependencies`, missing binary, bad `main`) that a pre-publish check structurally
  cannot see. What it gives up is the behavioral re-assertion, which the pre-publish gate
  already makes and which cannot gate publication from where it stands.
- Give each corrected requirement a mechanism that fails when the property it names is
  removed, rather than a check that resolves a path and reports success.

**Non-Goals:**

- Not changing the `formula-eval` feature gate, the opt-in default, or the published artifact
  contents. The feature behaves correctly; only the *verification* of the build parity is
  missing.
- Not building a general spec linter. The `spec-integrity` rules added here are stated as
  authoring rules for humans and reviewers, matching how the two existing rules in that
  capability are enforced — by review, not by a parser. A linter that cannot distinguish a
  satisfied agreement from an unenforced one would recreate the failure it is meant to catch.
- Not restructuring `release-verification`'s OIDC requirements. Those two were verified
  correct (`id-token: write` at line 206, `npm@11` at line 221, no token anywhere) and are
  explicitly out of scope.
- Not touching `streaming-write-incremental`'s deferred-status convention or its `## Status`
  block. That convention is applied consistently and cross-referenced from four places; it is
  a working pattern, not a defect.

## Decisions

### D1: Split the smoke test rather than reorder it

**Decision:** Add a pre-publish assertion that loads the downloaded platform binaries directly
via the hand-maintained `index.js` glue, and keep the existing `npm install` resolution as a
separate post-publish step with a narrowed claim.

**Rationale:** The current step is two independent checks fused into one:
1. *Does the built artifact behave correctly?* — answerable pre-publish from the local binaries.
2. *Does what npm serves resolve and behave correctly?* — only answerable post-publish.

Fusing them forces the weaker of the two to be late, which is what falsified the spec claim.
Splitting lets each carry a true claim at its own position.

**Alternative considered:** Reorder the whole step before publish. Rejected — it cannot work.
`npm install` of a not-yet-published version resolves to nothing, so the step would have to be
rewritten wholesale and would lose the packaging check that has real value.

**Alternative considered:** Change the claim to "after publish" and accept the weaker
guarantee. Rejected for the behavioral requirements, but adopted for the packaging check (D2) —
that is the honest claim for what npm resolution proves.

### D2: Narrow the post-publish step's claim to packaging

**Decision:** Rename the post-publish step to state that it verifies the published package set
resolves and loads from the registry, and reduce its body to those assertions. It no longer
re-asserts cell style, merged ranges, or row style — those are gated pre-publish.

**Rationale:** A check that runs after publication proves something real about publication.
Mislabeling it as a release gate is what made the corpus untrustworthy; labeling it accurately
costs nothing and loses no coverage.

**Correction made during review (the first draft kept every assertion):** renaming alone did not
make the requirement true. The post-publish step still asserted `font.bold`, `mergedRanges`, row
`font.color`, and `fill.foreground` *after* both publishes, so `release-verification`'s "every
behavioral assertion runs before the first `npm publish`" and its "no package SHALL have been
published by that run" scenario were both false on arrival — the exact defect class this change
exists to remove, shipped inside the change meant to remove it. A behavioral failure after
publish cannot un-publish, so it reports a release failure while the packages are already on npm.
Keeping those assertions bought no coverage the pre-publish gate did not already provide, because
both assert the same three guarantees through the same code. The requirement was correspondingly
narrowed to behavioral assertions, with packaging-only assertions after publication governed by
their own scenario.

### D3: Assert the output-phase bound, not input-phase streaming

**Decision:** The requirement states the streaming round-trip runs pre-publish against every
build-matrix target's own binary, over a workbook whose payload materially exceeds a spot check
and which exercises every cell value shape the streaming API accepts. The threshold lives in a
scenario. The requirement references `openspec/specs/streaming-xlsx` for the memory model rather
than restating it.

**Correction made during implementation:** the first draft of this delta demanded that the
smoke test detect "a regression that collects all sheets before writing". That contradicts
`streaming-xlsx`, which states the input phase's peak memory SHALL be **O(all sheets), NOT
constant**, and `WorkbookStreamXlsx.write(sheets: Array<JsStreamSheet>)` takes the whole sheet
array at the JS boundary by design. True incremental `writeSheet()` is deferred to
`streaming-write-incremental` and `ADR-028`. An assertion on the input phase could only ever
fail, or would have to be weakened until it passed — leaving the spec structurally valid and
semantically false, which is the disease this change exists to remove. The claim was narrowed
to the output phase, which is bounded today.

**Rationale for the scenario threshold:** `config.yaml` requires enumerated thresholds to live
in scenarios. It also lets the workbook grow without amending the requirement — the property
("payload exceeds what the output phase holds") stays true across size changes, whereas a
pinned row count does not.

**Scale choice:** Enough cell payload spanning multiple sheets that truncation, corruption,
and archive-format limits at scale are reachable, while staying inside the release runner's step
timeout. Measured against this build: 8 sheets × 6,000 rows × 5 cells = 240,000 cells, ~5.4 MB of
emitted cell content, ~4.9 s locally. The step was given an explicit `timeout-minutes: 5`; the
non-musl step previously had none and relied on the job-level cap.

**Correction made during review (value shapes):** raising the row count also *narrowed* coverage.
`main` asserted all four shapes the streaming API accepts — `number`, `text`, `boolean`,
`formula` — on its single row. The scaled-up version emitted only `number` and `text`, so
`StreamValue::Formula` was left with no release-path coverage and no unit test, while the delta
claimed the streaming assertions were "added alongside the in-memory ones rather than replacing
them". Scale and shape coverage are independent axes; growing one silently shrank the other. The
generator now writes and asserts all four shapes, asserts that an unwritten shape does not appear
on read-back (catching a writer that coerces `number` into `text`), and asserts up front that
every shape was actually produced, so the fidelity loop cannot pass vacuously.

**Correction made during review (the payload floor measured the wrong quantity):** the floor was
accumulated from each cell's template string, including column 1 — which is written as a
*number*, so its template text is never emitted. That overstated the payload by ~25% (7.43 MB
reported against 5.95 MB actually written), which is precisely the quantity the delta's "materially
larger than a spot check" scenario reasons about. The floor now sums per-shape emitted length:
digits for a number, `TRUE`/`FALSE` for a boolean, the expression text for a formula, the string
for text. The floor could previously be satisfied in part by bytes that never reached the archive.

**Why no memory assertion (second correction, from measurement):** the first rescope assumed
the output phase was observably bounded and asserted peak heap stayed flat relative to payload.
Measurement contradicts that. At fixed total payload (3.7 MB) peak heap growth during `write()`
is flat — 5.5 MB at 2 sheets, 5.2 MB at 128 sheets, a 62× change in per-sheet payload moving
nothing — while across increasing total payload growth rises ~1.5× linearly. Aggregate
JS-visible heap through `write(sheets) -> Buffer` is therefore O(total), not O(one sheet).
There is no bounded-output property to assert, so asserting one would produce a check that can
only fail. The requirement asserts scale and exact fidelity instead, which are both true and
falsifiable.

The genuinely bounded channel (cap 16, backpressure) belongs to `finalizeToReadable` and is
already owned by `streaming-write-to-readable`; it is not exercised by `write()` and is out of
scope here.

### D4: Feature parity is a script comparing files the pipeline already reads

**Decision:** Add `scripts/verify-feature-parity.cjs`, extracting `--features` values from
`package.json` scripts and from every `napi build` invocation in `.github/workflows/*.yml`,
failing when the sets differ. Wire it into CI alongside `verify-build-output`.

**Rationale:** Both files are already read by existing verification scripts
(`verify-build-output.cjs`, `verify-roadmap-drift.cjs`), so this follows the established
pattern rather than introducing a new kind of check. It reads the *declarations* rather than
inspecting a build, which is what makes it fast and hermetic.

**Alternative considered:** Have CI run `pnpm build` and diff resolved artifacts. Rejected —
slower, and it would prove the outputs agree rather than the *declared* feature sets agree,
which is what the requirement is about. The declarations are the thing that can silently drift.

**Corrections made during review (the first draft could pass while the invariant was broken):**
the checker compared one feature set per workflow *step chunk*, with chunk boundaries matched at a
fixed six-space indent. Executing it against mutated copies of `.github/` showed three silent
passes:

- *A source disappears.* Deleting `release.yml` left "every invocation that exists agrees"
  trivially true. `REQUIRED_SOURCES` now asserts `package.json`, `ci.yml`, and `release.yml` are
  all present, so absence is a failure rather than a smaller comparison set.
- *A step is re-indented.* An eight-space-indented `napi build` with no `--features`, placed after
  a `cargo test --features formula-eval` step, merged into that chunk and inherited its feature
  set — reported `OK` while the build declared nothing. Boundaries are now matched at any indent,
  and, more importantly, each `napi build` occurrence is compared on its own rather than per
  chunk, so the same holds when two builds share one step.
- *All flags are stripped.* Every source dropping `--features` yields mutual agreement on the
  empty set, which passed. An empty reference set is now a failure: every `napi build` in this
  project declares at least one feature, so agreement on nothing is a degenerate comparison.

A YAML block scalar folds newlines into the command, and `--features \` continues the value onto
the next line; the first draft read the trailing `\` as a feature literally named `\`. Values are
now flattened across continuations before parsing.

Note the asymmetry these fixes preserve: a *disagreement* in any direction fails, a missing source
fails, and a degenerate empty agreement fails — while legitimate reformattings (quoted
`--features=`, backslash continuation, a feature added consistently to every source, an unrelated
new workflow) still pass. Each case is covered by a probe that asserts the mutation applied before
recording the result, so a broken probe cannot read as a passing check.

### D5: Correct the ADR citations in the specs, not the ADR

**Decision:** Change the two `ADR-005` references in `streaming-write-incremental` to name the
streaming write buffering decision, and extend `design-record`'s citation rule to cover
renumbering.

**Rationale:** `ADR-028` and its numbering note are already correct — they record the
005→028 renumbering and why. Only the citing documents are wrong. The rule is extended because
the existing rule ("cites a standing one") was *satisfied* by `ADR-005`, which is `current`
and concerns licensing. Adding the renumbering clause closes the gap without inventing a
mechanism: this is a review-enforced authoring rule, consistent with the rest of
`design-record`.

**Note:** A path-resolution script would **not** catch this. `docs/adr/005-*.md` exists and is
`current`; resolution and relevance are independent properties. This is why no citation linter
is proposed — the bug class is semantic, and review is the honest enforcement.

### D6: Invert the musl loading requirement to what dynamic linkage actually provides

**Decision:** Rewrite the `platform-targets` requirement *musl binaries load without a
host-matched libc*. It claimed a dynamically linked musl binary "loads in any Node.js process
regardless of the host's libc", with a scenario asserting a successful round-trip on a
non-musl-glibc host. Dynamic musl linkage is precisely what makes that impossible: the
cdylib records its musl libc as a dynamic dependency, so a glibc host cannot resolve it.

**Evidence:** `readelf -d` on `excelrs.linux-x64-musl.node` reports `NEEDED libc.so` (the
`libc.musl-<arch>.so.1` name is the musl *loader's* SONAME, not the dependency recorded by
the cdylib). Loading it in `node:20-bookworm-slim` (glibc 2.36) fails with `libc.so: cannot
open shared object file`. The mirror case — aarch64 binary on x86_64 — reports `unsupported
relocation type 1026`, and the same binary on aarch64 loads and round-trips.

**Rationale:** The requirement's own title and rationale were also inverted: it required
linking *to* a libc under the heading "load without a host-matched libc". The real reason
dynamic linkage is mandatory is already recorded in `release.yml` and `CHANGELOG.md` — a
fully static musl cdylib **segfaults** on `dlopen` (static TLS/pthread init collides with the
host libc) — and the release's existing `Assert musl binary links musl libc` step already
rejects a fully static build by failing when no `NEEDED` entry is present. So the corrected
requirement keeps an enforced obligation, drops the false one, and gains a scenario for the
non-musl host that the loader error actually produces. Routing away from a non-musl host is a
consumer-side decision the loader makes from the host libc and architecture, not a property
the binary provides.

### D7: Report the load-failure cause chain, not the loader's outer message

**Decision:** `scripts/streaming-smoke.cjs` printed only `err.message`'s first line on a load
failure. Print the `err.cause` chain instead.

**Rationale:** `index.js` collapses every load failure into one `Cannot find native binding …
npm has a bug related to optional dependencies … rm -rf node_modules` message and keeps the
actual reason only in a chained `cause`. On a real failure the outer line blames npm and the
inner line is the truth. Walking the chain is a bounded loop over an existing field; no new
diagnostic mechanism is introduced. The same reasoning was already applied to
`prepublish-smoke.cjs` (task 2.6) and to `musl-smoke-test.cjs`, which had been left with an
unguarded `require` on the one musl path that can actually hit an architecture mismatch.

## Risks / Trade-offs

**[Pre-publish gate fails on a locally-loadable artifact but the published one still breaks]**
→ The post-publish npm-resolution check (D2) is retained unchanged. The two checks cover
different failure modes; neither subsumes the other.

**[A 2-minute release timeout is insufficient for the scaled streaming round-trip]**
→ Calibrate the row count in the implementing task against the actual runner, and raise the
step's `timeout-minutes` if needed. The property under test (buffering is detectable) degrades
gracefully — a smaller workbook still catches a regression that collects *all* sheets, just
with less headroom.

**[The pre-publish step must load the correct binary per matrix target]** → The `publish` job
runs on `ubuntu-22.04` after downloading all six artifacts; loading a foreign-target binary is
not generally possible. → **Mitigation:** the pre-publish assertion runs the ubuntu-x64-gnu
binary. Per-target verification of the *other five* targets stays in the `build` job, which is
where `platform-targets`' per-target requirement already points once corrected.

**[Feature-parity script parses YAML without a YAML dependency]** → `package.json` is JSON;
the workflow side needs `--features` extraction from `*.yml`. → **Mitigation:** match the
`--features` token with a narrow regex rather than parsing YAML, so no parser dependency is
added. Acceptable because the assertion is narrow and its failure mode (a flag written in an
unmatched form) is a visible test failure, not a silent pass.

**[Adding rules to `spec-integrity` without enforcement adds prose, which was the original
disease]** → The two new rules restate a principle already present twice in that capability.
They are written to be checkable-by-review and name no mechanism that does not exist. If the
corpus later wants enforcement, that is a separate change with its own cost discussion.

## Migration Plan

No consumer-facing migration. No API, artifact, or package-contents change. The published
`formula-eval` artifacts are byte-identical before and after; only the workflow's step
ordering and one script's scale change.

Rollback is a revert of the workflow and script edits. The spec deltas would be archived, so
reverting after archive means a follow-up change — hence the sequencing in `tasks.md` puts
workflow changes behind spec edits within the same change.

## Open Questions

None that affect the specs, approach, or task breakdown. The two candidates considered and
resolved rather than deferred:

- *Should `merge-range-parsing-consolidation` be deleted in this change?* No — it is a
  capability-taxonomy defect, not a process-claim defect. Mixing them would make the change
  hard to review and would couple an unrelated precedent (`97517e3`'s retirement of
  `musl-support`) to release-pipeline work.
- *Should the two fossils (`date-cell-value:62`, the `release-verification:15` scenario) be
  removed here?* No — same reason. They are unrelated spec-text cleanups.