# Formula Corpus Analysis — `typework-formula-corpus` spike

**Date:** 2026-08-08
**Owner:** excelrs formula team
**Input to:** issue #51 lookup decision (`add-formula-lookups` go/no-go)
**Tool:** [`examples/analyze_corpus.rs`](../examples/analyze_corpus.rs)

---

## TL;DR — Recommendation

**Defer the lookup build. Do not open `add-formula-lookups` yet.**

- The typework corpus was **not obtainable** (see Corpus below). There is zero
  in-repo evidence that any real consumer needs lookups (`grep` for
  `typework` / `wopi` / `onlyoffice` returns nothing).
- The only real workbooks available offline are a **curated formula-eval test
  suite**, which is engineered to stress *unsupported* functions — so its
  function frequencies are **not representative** of typework demand and cannot
  drive a go/no-go decision.
- The immediate, shippable value from #51 — **fresh-value recalculation** —
  has already landed (PR #62, `Workbook::recalculate` / `Worksheet::recalculate`).
- A **reusable analysis tool now exists**. The moment typework supplies
  representative workbooks, one command produces the real analysis.

> This spike's durable output is the **baseline capability table** + the
> **committed tool**, not a number. The decision is intentionally left
> data-blocked rather than fabricated.

---

## 1. Baseline Capability Table (Tasks 1–2)

Shipped evaluator: `src/formula/bridge.rs`, `call_function`.

### Shipped (18 callable function names)

`SUM, AVERAGE, MIN, MAX, COUNT, COUNTA, IF, AND, OR, NOT, ABS, ROUND,
CONCATENATE (= CONCAT), LEFT, RIGHT, MID, LEN, IFERROR`

> `CONCATENATE` and `CONCAT` resolve to the **same dispatch arm** (`"CONCATENATE"
> | "CONCAT"`), so they are one function, not two.
> `TRUE` / `FALSE` are **parser literals**, not function-call names — they are
> supported as boolean values but never reach `call_function`.

### Correction to the proposal's "21 functions"

The proposal lists 21 by counting `CONCAT` and `CONCATENATE` separately *and*
including `TRUE`/`FALSE` as literals. The **dispatch actually handles 18
distinct callable names**; the rest of the "21" is a double-count + literals.
This spike uses the code-accurate **18**.

### Gap set (named in #51 — absent)

| Function  | Kind            | Why expensive                          |
|-----------|-----------------|----------------------------------------|
| `INDEX`   | 2D range return | array returns, implicit intersection   |
| `MATCH`   | lookup helper   | 2D range semantics                     |
| `XLOOKUP` | large arg surface | many optional args, array semantics  |
| `VLOOKUP` | lookup          | range scan + approximate-match modes   |

Broader unsupported reference family (surfaced for awareness, not in #51's
named set): `HLOOKUP, LOOKUP, CHOOSE, OFFSET, INDIRECT, XMATCH, FILTER, SORT,
UNIQUE, TRANSPOSE`.

---

## 2. Corpus (Tasks 3–4)

| Priority | Source | Status |
| ---------- | -------- | -------- |
| 1 (ideal) | Workbooks typework serves over WOPI `GetFile` | **Unavailable** — cannot contact the typework team from this environment; repo contains no typework/wopi/onlyoffice references. |
| 2 (fallback) | Public Excel report/template workbooks | Not downloaded (binary fetch blocked in this sandbox). |
| offline-only | `xlstream-eval` crate test fixtures (real `.xlsx`) | **Used for pipeline validation only** — see caveat. |

### Sample

- **N = 89 real `.xlsx` files**, 3,325 formula cells, across all sheets.
- **Source path:** `~/.cargo/.../xlstream-eval-0.4.0/tests/fixtures/**` (a
  formula-evaluation library's regression suite).
- **Sample size:** 89 workbooks / 3,325 formulas.

> ⚠️ **Representativeness caveat (critical).** These fixtures were written to
> *exercise* formula functions, including the lookup family. They therefore
> **over-represent** advanced/unsupported functions relative to a typical
> business report. Their frequencies are **directional at best and biased by
> construction** — they must not be read as evidence of what typework's
> workbooks use. They are used here only to (a) validate the tool and (b)
> characterize the engine's *capability ceiling*.

---

## 3. Methodology (Tasks 5–8)

`examples/analyze_corpus.rs` (committed):

1. Walk a directory for `*.xlsx` (recursive).
2. Open each with `calamine`; for every sheet call `worksheet_formula` and
   collect non-empty formula strings.
3. Tokenize every `NAME(` function call; record the **leading** function of
   each formula.
4. Classify each formula:
   - **fully evaluable** = all referenced functions are in the shipped 18.
   - **needs missing lookup** = references ≥1 of `INDEX/MATCH/XLOOKUP/VLOOKUP`.
   - **needs broader unsupported** = references another unsupported function.
   - **cross-sheet** = formula text contains `!` (a `Sheet!A1` qualifier).
5. Emit JSON (captured into this report) + a stderr summary.

---

## 4. Results (curated-eval corpus — NOT representative)

| Metric | Value |
| -------- | ------- |
| Total formulas | 3,325 |
| Cross-sheet (`Sheet!A1`) | 224 (6.7%) |
| Fully evaluable by shipped engine | 899 (27.0%) |
| Needs a missing #51 lookup | 111 (3.3%) |
| Needs other unsupported function | 24 (0.7%) |

> The 27% "fully evaluable" figure is **by construction**: the suite is built
> to populate the other 73% with unsupported functions. It says nothing about
> typework.

### Ranked function frequency (top, all formulas)

| Function | Shipped? | Formulas containing |
| ---------- | ---------- | --------------------- |
| IF | ✅ | 125 |
| IFERROR | ✅ | 118 |
| SUM | ✅ | 89 |
| VLOOKUP | ❌ (gap) | 79 |
| COUNTIF | ❌ | 67 |
| SUMIF | ❌ | 58 |
| DATE | ❌ | 46 |
| ABS | ✅ | 40 |
| CONVERT | ❌ | 39 |
| MAX | ✅ | 30 |
| … | | |
| INDEX | ❌ (gap) | 24 |
| MATCH | ❌ (gap) | 16 |
| HLOOKUP | ❌ | 12 |
| CHOOSE | ❌ | 12 |

Full tallies (shipped + gap):

- Shipped present: `IF 125, IFERROR 118, SUM 89, ABS 40, MAX 30, AVERAGE 29,
  MIN 29, ROUND 26, LEFT/RIGHT/MID/LEN 20 each, etc.` `CONCAT` = 0 (only
  `CONCONCATENATE` used in fixtures).
- Gap set: `VLOOKUP 79, INDEX 24, MATCH 16, XLOOKUP 0 (see note)`.

> **Extraction caveat:** counts come from calamine `worksheet_formula`. One
> fixture (`lookup/xlookup.xlsx`) stores `XLOOKUP` in a form calamine did not
> surface, so **XLOOKUP is under-counted here** (it is genuinely present in the
> file). VLOOKUP/INDEX/MATCH tallies are reliable. The committed tool is
> sufficient for typework's normally-authored workbooks; this is a fixture
> edge case.

### Leading-function distribution (top)

`IF 115, IFERROR 110, COUNTIF 67, SUM 59, SUMIF 58, VLOOKUP 55, CONVERT 38,
CEILING 31, FLOOR 30, MAX 30, TEXT 30, MIN 29, SUMIFS 28, AVERAGE 27, …`

### Cross-sheet (validates `expose-formula-recalc`)

6.7% of formulas reference another sheet. This independently confirms the
cross-sheet recalc need that **`expose-formula-recalc` (#62) already shipped**.
No new work required there.

---

## 5. Missing-lookup impact

- In the curated suite, only **3.3% of formulas** reference one of the four
  named lookups — but that suite is built to include them, so this is an
  *upper-ish* bound from a biased sample, not a real demand figure.
- The expensive/uncertain work in #51 is precisely the lookup family. Building
  it speculatively, with **no evidence any real consumer needs it**, is the
  risk the spike was opened to prevent.

---

## 6. Decision & next step (Task 11)

**Fate of #51:** Keep open as *"fresh-value recalc shippable (#62 merged);
lookups deferred pending corpus evidence."* **Do not** open
`add-formula-lookups`.

**To unblock the decision (when typework engages):**

1. Obtain ~5–10 representative workbooks typework serves over WOPI `GetFile`.
2. `cargo run --example analyze_corpus -- <dir> "typework-<date>"`
3. Read the emitted `fullyEvaluablePct` / `needsMissingLookupPct` against the
   bar chosen below.

**Go/no-go bar (resolve before writing the final recommendation):**

- *Conservative:* build lookups only if a meaningful share of real formulas
  need them (e.g. >5–10%).
- *Liberal:* any real lookup usage justifies the build (correctness floor for a
  general Excel engine).
This spike takes **no position** — the corpus needed to choose is absent.

**Link this report from #51** once the issue is reachable.

---

## Reproduce

```bash
cargo run --example analyze_corpus -- <corpus-dir> "<label>"
# prints JSON to stdout; stderr shows file count + corpus label
```

*Sample size and source are stated above so the recommendation is auditable.*
