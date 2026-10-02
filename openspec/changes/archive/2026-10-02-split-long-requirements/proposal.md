# Proposal

## Why

Strict OpenSpec validation reports 22 INFO-level findings — requirement text over 500
characters — across 12 of the project's 41 capabilities. Every one of these requirements
bundles several independent behaviors, a long enumeration, or a repeated deferred-scope
disclaimer under one `### Requirement` heading. That makes the requirements harder to review
against the code, and it is the last category of finding the corpus still carries after the
`reconcile-parity-spec-drift` change closed the ERROR and WARNING findings.

The findings are advisory today and do not fail CI. This change makes them actionable by
rewriting the requirement prose so each requirement states one behavior, while preserving
every behavior, scenario, and requirement that exists today.

## What Changes

- Split 22 over-long requirements across 12 capabilities into requirements that each state a
  single behavior, so every requirement's prose drops below the 500-character threshold.
- Split **enumeration-driven** requirements (per-rule-type field maps, per-font-attribute
  resolution rules, the `CfRule` type list, the npm package list) into one requirement per
  enumerated item, with each item's field/attribute map moved into its own scenario.
- Split **multi-behavior** requirements (the `streaming-xlsx` writer's three output targets,
  the `addImage` anchor + EMU + Buffer-typing contract, the rich-text per-attribute reader
  contract) into one requirement per behavior.
- Consolidate the four restatements of the streaming input-phase / output-phase deferred-scope
  disclaimer into a single normative statement in `streaming-xlsx`, with the other three
  streaming specs holding a short cross-reference to it. This keeps the caveat once, in one
  place, rather than reworded four times.
- Preserve all existing behaviors, scenarios, requirement names that remain meaningful, and
  the `streaming-write-incremental` spec exactly as-is; this change is prose-only and
  introduces no behavior, API, or code change.
- Non-goal: this change does not decide whether `streaming-write-incremental` belongs in
  `specs/`; it only cross-references it.

## Capabilities

### New Capabilities

None. This change restructures existing requirements; it introduces no new capability.

### Modified Capabilities

All 12 existing capabilities whose requirements are being rewritten. Each uses its exact
existing path under `openspec/specs/`:

- `cached-formula-value`: split the two over-long requirements (cached `<v>` emission +
  `t`-attribute priority; `cachedValue` recalc-only contract) into one-behavior requirements.
- `conditional-formatting`: split the `CfRule` rule-type field map and the writer's
  `conditionalFormatting` + `dxfs` emission into one-behavior requirements.
- `date-cell-value`: split the UTC-anchored serial mapping from the JS `Date` assignment rule.
- `exceljs-parity`: split the feature-area enumeration and the v2.0.0 completion record into
  per-area / per-topic requirements without dropping any listed area or exclusion.
- `images`: split the `addImage` contract into anchor shape, anchor inference, EMU offset
  math, and Buffer typing/runtime requirements; split the writer media/drawing requirement.
- `release-verification`: split the seven-package OIDC trusted-publishing requirement into the
  auth model, the package set, and the spec/workflow parity rule.
- `rich-text`: split the reader's per-font-attribute contract, the shared-strings writer
  requirement, and the cross-app verification requirement into one-behavior requirements.
- `streaming-write-incremental`: split the `finalizeToReadable` cancelable/self-cleaning
  requirement into the termination guarantee, the live-consumer guarantee, and the bridge
  teardown rule. The capability itself is unchanged and stays in `specs/`.
- `streaming-write-to-file`: split the file-path finalize requirement into the API contract
  and the output-phase backpressure guarantee; the deferred-scope caveat becomes a cross-ref.
- `streaming-write-to-readable`: split the `ReadableStream` requirement into the API contract
  and the output-phase backpressure guarantee; the deferred-scope caveat becomes a cross-ref.
- `streaming-xlsx`: split the writer's output-targets requirement into per-target contracts,
  consolidate the deferred-scope disclaimer here, and split the reader's resource-bounding and
  shared-formula requirements.
- `tables`: split the `addTable`/`getTable`/`getTables`/`removeTable` surface and its options
  into per-operation requirements without dropping any option or the uniqueness rule.

## Impact

- **Specs only.** 12 files under `openspec/specs/` are rewritten at the requirement-prose
  level. No new capability directory is added and the corpus stays at 41 capabilities.
- **Requirement counts rise.** Splitting increases per-capability `### Requirement` counts;
  the spec inventory will report higher counts for all 12 capabilities. This is expected and
  is not a behavior change.
- **No code, API, or dependency change.** No Rust, JavaScript, native binding, or published
  package behavior is touched.
- **CI.** The existing strict-validation gate keeps passing; the change is intended to move
  the corpus from 22 long-requirement INFO findings toward zero, with a documented allowance
  for any enumeration that cannot be split without losing meaning.
- **Downstream.** The archive path for this change will sync the rewritten requirements into
  `openspec/specs/`. Because requirement names may change during splitting, the implementer
  MUST keep each split's scenario coverage intact and verify with
  `openspec validate --strict --specs` after syncing.