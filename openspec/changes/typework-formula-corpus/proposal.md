## Why

The formula engine (issue #51) shipped SUM/IF + 19 other functions but is
missing the lookup family the issue named "at minimum" alongside them:
`INDEX`, `MATCH`, `XLOOKUP`, `VLOOKUP`. Building those is the most expensive,
least-certain work in #51 — they require 2D range semantics, array returns,
implicit intersection, and (XLOOKUP) a large argument surface. There is
currently zero evidence in this repo that any real consumer (typework's WOPI
host, or otherwise) actually needs them: a repo-wide grep for `typework` /
`wopi` / `onlyoffice` returns nothing, and `Worksheet.recalculate` is not even
JS-exposed yet (see change `expose-formula-recalc`).

Before investing in lookups, we need a corpus-based answer to one question:
**what functions do the workbooks typework actually serves over WOPI
`GetFile` use, and what fraction can the shipped engine already evaluate?**
This spike produces that evidence so the lookup-build decision is data-driven,
not speculative.

## What Changes

- This is an investigation, not a code change. No system behavior changes.
- Deliverable: a corpus analysis (markdown report) listing, per function,
  frequency across representative typework workbooks, plus the share of
  formulas the current engine can fully evaluate vs. those that would
  `#NAME?`/`#REF!` without lookups.
- Output feeds a go/no-go decision for a follow-up `add-formula-lookups`
  change (or a decision to close #51 as "fresh-value recalc shippable;
  lookups deferred").

## Capabilities

Pure investigation — no spec-level behavior changes. `skip_specs: true` is
declared in `.openspec.yaml`; no delta specs are created.

### New Capabilities

(none)

### Modified Capabilities

(none)

## Impact

- No code, dependency, API, or ABI impact.
- Consumes a small corpus of representative `.xlsx` files (provided by the
  typework team, or sourced from public Excel report templates as fallback).
- Resulting report is input to the #51 lookups decision and the
  `expose-formula-recalc` follow-up prioritization.
