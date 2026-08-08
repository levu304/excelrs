## 1. Establish the baseline capability table

- [x] Enumerate the 21 shipped functions from `src/formula/bridge.rs`
      (`SUM, AVERAGE, MIN, MAX, COUNT, COUNTA, IF, AND, OR, NOT, ABS, ROUND,
      CONCAT, CONCATENATE, LEFT, RIGHT, MID, LEN, IFERROR, TRUE, FALSE`).
- [x] List the 4 missing lookups named in #51 (`INDEX, MATCH, XLOOKUP,
      VLOOKUP`) as the gap set.

## 2. Acquire the corpus

- [x] Request ~5-10 representative `.xlsx` workbooks from the typework team
      (the WOPI `GetFile` files they serve).
- [x] If unavailable, source public Excel report/template workbooks as fallback
      (note the source + sample size in the report).

## 3. Extract formulas

- [x] For each workbook, read all sheets and collect every formula string from
      `<f>` elements (reuse excelrs `Workbook` read, or calamine).
- [x] Record total formula count and which sheets/formulas are cross-sheet
      (`Sheet2!A1`) — this validates the `expose-formula-recalc` cross-sheet need.

## 4. Analyze function usage

- [x] Tokenize the leading function name of each formula; tally absolute
      frequency and % of all formulas per function.
- [x] Compute coverage: % of formulas fully evaluable by the shipped engine
      (all referenced functions present) vs. % that would fail without the
      missing lookups.

## 5. Write the report

- [x] Produce `docs/formula-corpus-analysis.md` (or a file in this change dir)
      with: ranked function-frequency table, missing-lookup impact (how many
      formulas break), cross-sheet frequency, and a one-line recommendation
      (build lookups / defer / close #51).
- [x] State sample size and source so the recommendation is auditable.

## 6. Feed the decision

- [x] Use the report to decide the fate of #51 and whether to open an
      `add-formula-lookups` change. Link the report from #51.
