## Context

The shipped evaluator (src/formula/bridge.rs) supports ~21 functions: SUM,
AVERAGE, MIN, MAX, COUNT, COUNTA, IF, AND, OR, NOT, ABS, ROUND, CONCAT,
CONCATENATE, LEFT, RIGHT, MID, LEN, IFERROR, TRUE, FALSE. The lookup family
(INDEX, MATCH, XLOOKUP, VLOOKUP) named in issue #51 is absent. This spike
establishes demand before building them. See proposal.md — Why.

## Goals / Non-Goals

Goals: produce a ranked, frequency-based view of formula-function usage in
representative real-world workbooks, and the coverage gap versus the shipped
engine.

Non-Goals: do not build any functions; do not modify the evaluator; do not
generalize beyond the "should we build lookups" decision.

## Decisions

1. **Baseline capability table first.** Enumerate the 21 shipped functions and
   the 4 missing lookups as the reference set.
2. **Corpus sourcing (in priority order):**
   1. Workbooks typework serves over WOPI `GetFile` (most authoritative — ask
      the typework team for ~5-10 representative files covering their common
      report shapes).
   2. Fallback: public Excel report/spreadsheet templates (e.g. financial /
      inventory / dashboard templates) if typework files are unavailable.
3. **Extraction method.** Parse each `.xlsx` (reuse excelrs `Workbook` read, or
   calamine directly) and collect every formula string from `<f>` elements
   across all sheets.
4. **Analysis.** Tokenize each formula's leading function name(s); tally
   frequency (absolute + % of all formulas). Flag formulas referencing
   cross-sheet (`Sheet2!A1`) to confirm the cross-sheet-recalc need already
   covered by `expose-formula-recalc`. Compute coverage: % of formulas fully
   evaluable by the shipped engine (all referenced functions present) vs. % that
   would fail without lookups.
5. **Output artifact.** A markdown report (e.g. `docs/formula-corpus-analysis.md`
   or the change dir) with: ranked function-frequency table, missing-lookup
   impact (how many formulas break), and a one-line recommendation
   (build lookups / defer / close #51).

## Risks / Trade-offs

- [Corpus not representative] → Mitigation: prefer typework's own files; note
  sample size and source in the report so the recommendation is auditable.
- [Small N] → Mitigation: state sample size; treat as directional, not
  definitive.
- [Formula extraction misses array/dynamic spills] → Mitigation: count by
  leading function token; acceptable for frequency ranking.

## Migration Plan

N/A — investigation only. The report is the deliverable; no rollback needed.

## Open Questions

- Can the typework team supply representative workbooks, or should we use
  public templates as fallback?
- Is "fraction of formulas the engine can evaluate" the right go/no-go bar, or
  is any lookup usage (even rare) sufficient justification? (Decide before
  writing the recommendation.)
