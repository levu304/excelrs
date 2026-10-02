# Design

## Context

See proposal.md — Why, for motivation. The relevant current state:

Strict validation reports 22 INFO-level findings — requirement prose over 500 characters —
across 12 of the 41 capabilities. Every flagged requirement already has scenarios
(1–10 each); what is long is the normative prose above the first `#### Scenario:`. Measured
prose lengths range from 579 to 1,959 characters (median 770, total ~19.3k characters).

Three distinct causes are present, and they call for different treatments:

1. **Multi-behavior requirements** — several independent behaviors bundled under one
   `### Requirement` heading. The `streaming-xlsx` writer requirement (1,959 chars) alone
   carries the two-phase memory model plus three separate output-target contracts.
2. **Repeated deferred-scope disclaimer** — the streaming input-phase/output-phase
   explanation is restated, each time reworded, in four requirements across three specs.
   A paragraph-level duplicate scan over the 22 findings found no *verbatim* duplication, so
   the repetition is genuine rather than copy-paste.
3. **Inherent enumerations** — flat lists that are the requirement's content
   (`exceljs-parity`'s feature areas, `tables`' option surface, `release-verification`'s
   package list).

Constraints shaping the approach:

- `openspec instructions specs` states that under `MODIFIED Requirements` the existing
  requirement block must be kept whole and never split or trimmed for length. Splitting a
  requirement therefore cannot be expressed as MODIFIED.
- The validator rejects a change whose delta lists the same requirement name under both
  `ADDED` and `REMOVED` ("Requirement present in both ADDED and REMOVED"). A split that
  preserves the original name for one of its parts is therefore not expressible.
- Every requirement in a delta needs at least one `#### Scenario:` (4 hashtags).
- `spec-integrity` requires main specs to keep canonical structure and requires the
  `recalc_only` flag-style implementation detail to stay out of observable contracts —
  relevant because one split requirement carried that flag in its prose.

## Goals / Non-Goals

**Goals:**

- Get every affected requirement's prose under 500 characters, addressing all 22 findings.
- Preserve every behavior, scenario, enumerated item, and numeric threshold present today.
- Keep each requirement stating one behavior, so the split has review value beyond the
  character count.
- State the streaming two-phase memory model once, normatively, with the other specs
  referencing it.
- Keep the capability inventory at 41 — no new capability directory, no capability moves.

**Non-Goals:**

- Deciding whether `streaming-write-incremental` belongs in `specs/`. It stays; this change
  only cross-references it.
- Any code, API, dependency, or runtime behavior change.
- Tightening the 500-character guideline into an enforced ERROR, or converting the INFO
  findings into build failures.
- Rewriting `exceljs-parity`'s matrix content or re-granularizing the matrix itself.

## Decisions

### D1: Express splits as REMOVED + ADDED rather than MODIFIED

A split removes one requirement and adds several. MODIFIED cannot express this — the
instruction requires the existing block stay whole, and the validator requires header names
to match exactly.

*Alternative considered:* MODIFIED with the original name carrying the primary behavior and
ADDED carrying the rest. Rejected: the validator errors when a name appears in both ADDED and
REMOVED, and using MODIFIED to shrink the original block contradicts the "never trim or
rewrite existing text to meet the length" instruction.

*Consequence:* every requirement in the delta whose name is preserved verbatim from the main
spec gets a distinguishing qualifier so it does not collide with its REMOVED entry (e.g.
`Worksheet exposes image add/get` → `Worksheet exposes addImage and getImages`). This makes
the rename visible in review, which is the point.

### D2: REMOVED entries carry Reason and Migration

Each REMOVED requirement carries a **Reason** (why the requirement is being replaced) and a
**Migration** pointer naming the successor requirements. Without these, a reader diffing the
main spec after archive cannot tell that behavior was preserved rather than dropped.

### D3: Move enumerated field/attribute maps into scenarios, not prose

Where a requirement enumerates items with per-item fields (the `CfRule` type→field map, the
rich-text per-font-attribute resolution rules), the enumeration is split: one requirement per
group, with each item's field details expressed as scenarios.

*Alternative considered:* keep the enumeration as a markdown table inside the prose. Tables
are compact and readable, but a 13-row table is still over 500 characters, and the whole
point is one behavior per requirement.

### D4: Split enumerations by grouping rather than one requirement per item

For genuinely flat lists — `exceljs-parity`'s feature areas, `tables`' option surface,
`release-verification`'s package list — this change splits into a small number of grouped
requirements rather than one per item. One requirement per ExcelJS feature area (19) or per
npm package (7) would multiply the inventory without making any requirement clearer.

*Trade-off:* the `exceljs-parity` area list itself is over 500 characters as a markdown list.
It is split by concern — the requirement states the areas, and the styling sub-areas, the
exclusions, and the status-tracking rule each get their own scenarios or requirements. The
area list is retained in full.

### D5: State the streaming two-phase model once in `streaming-xlsx`

`streaming-xlsx`'s "Streaming writer emits a workbook to a byte stream" is renamed to
"Streaming writer buffers input sheets and streams the output phase" and becomes the single
normative statement of the input/output phase model, including the ADR-005 deferral.
`streaming-write-to-file` and `streaming-write-to-readable` replace their restatements with
a cross-reference to that requirement.

*Rationale:* the three restatements are reworded rather than duplicated, which is why
deduplication tooling would not catch them, and why they must be replaced by reference rather
than by deletion.

*Trade-off:* the caveat is now one hop away in two capabilities. Acceptable, because the
referring requirements still state the operative consequence for their own target (O(all
sheets) peak memory), so a reader of either spec learns the thing that matters to them.

### D6: Keep `streaming-write-incremental`'s requirements intact but restructured

Its one flagged requirement is split into worker-termination, live-consumer delivery, and
bridge teardown. The capability stays in `specs/` and keeps its role as the target for
deferred true incremental `writeSheet()`; only the prose is restructured, and its requirement
names are renamed to avoid the ADDED/REMOVED collision.

### D7: Prefer scenario-preserving splits over scenario invention

Every successor requirement MUST carry at least one scenario, and the change MUST NOT drop
any existing scenario's assertion. Where the original requirement had one scenario covering
several behaviors, that scenario is distributed across the successors it was split into.

*Risk noted:* writing scenarios that assert behavior the original prose stated but no
original scenario covered. Mitigation in tasks: each split is reviewed against the original
requirement's full text, not just its scenarios.

### D8: Do not touch `spec-integrity`

`spec-integrity` has no flagged requirements and its three requirements are unaffected. This
change must not weaken the canonical-structure requirement, and must not introduce a new
spec-level rule (such as a hard 500-character limit) that would constrain future changes.

## Risks / Trade-offs

- **Behavior silently dropped during a split** → Mitigation: for each split, diff the union
  of successor requirement texts and scenarios against the original requirement text, and
  check every enumerated item and numeric threshold (`MAX_ENTRY_BYTES` 16 MiB,
  `MAX_EVENTS` 5,000,000, cap 16, the seven package names) survives verbatim. This is the
  primary correctness check for the change.

- **Requirement-name churn makes review harder** → Mitigation: the rename is forced by the
  validator (D1) and is confined to the affected capabilities; each REMOVED entry names its
  successors so the mapping is explicit.

- **Archive-time loss from MODIFIED-with-partial-content** → Mitigation: not applicable — no
  MODIFIED requirements are used. Every delta is REMOVED (whole original block) plus ADDED
  (whole successor blocks).

- **Cross-reference to `streaming-xlsx` creates a fragile coupling** between the two
  finalize specs and the model requirement → Accepted. The reference is by requirement name
  within the same corpus, and the referring requirements restate their own operative
  consequence, so a rename in `streaming-xlsx` would surface as a stale reference in review
  rather than a silent behavioral gap.

- **Splitting enumerations increases requirement counts and makes the inventory noisier** →
  Accepted as the cost of one-behavior-per-requirement. The inventory still reports 41
  capabilities; only per-capability requirement counts rise.

- **Some enumerations remain over 500 characters even after splitting** → Accepted and
  documented in the tasks' acceptance criteria as an allowance, rather than forcing a split
  that would lose meaning.

- **Scope creep into the `streaming-write-incremental` category question** → Mitigation: it
  is an explicit non-goal; the change only restructures that spec's prose and adds references
  to it.

## Migration Plan

1. Rewrite each of the 12 delta spec files, one capability at a time.
2. Validate the change: `openspec validate "split-long-requirements" --strict`.
3. Apply: sync the deltas into `openspec/specs/` and re-run
   `openspec validate --strict --specs` to confirm the corpus is still valid and that the 22
   long-requirement findings are gone.
4. Confirm the corpus stays at 41 capabilities and no capability was added or moved.

Rollback: revert the spec-file commits. There is no code, migration, or data impact; the
change touches only `openspec/specs/`.

## Open Questions

None. The scope decision (all 22 findings) and the `streaming-write-incremental` disposition
(stay, cross-reference only) were settled with the user before planning.