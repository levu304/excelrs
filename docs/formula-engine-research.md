# Research: Rust Formula Engines for excelrs

> ⚠️ **STATUS UPDATE — STALE.** This research (written ~July 2026) recommends `formularizer-eval` as "the single candidate." That crate family — and its GitHub repo `psu3d0/formularizer` — have since been **yanked/removed from crates.io (404) and the repo is gone (404)**. Do *not* follow this doc's engine recommendation. The shipped outcome (commit `82b7753`, branch `feat/formula-eval-integration`) **diverges**: it uses `xlstream-parse` (wrapping the surviving `formulizer-parse` parser) + `xlstream-core` + a **handwritten** `FormulaEvaluator` (`src/formula/bridge.rs`, ~560 lines, 20 built-ins, no Arrow). See `CHANGELOG.md` (`[Unreleased]`) and `openspec` `add-cached-formula-evaluation/design.md`. The analysis of *integration shape* (resolver-injection, the data-model seam) remains accurate; only the *crate recommendation* is dead.

Status: **exploratory** — not a commitment to implement. Maps the ecosystem against
excelrs's existing architecture and ADR-9 (`docs/spec.md` §10 / Appendix A #9).

ADRs in scope:

- **ADR-9** — "Formula evaluation... separate, massive undertaking (Excel supports
  500+ functions complex semantics). If evaluation ever added, will via integration
  with an existing Rust formula engine, not built from scratch."
- **Non-Goal §10** — "Formula evaluation engine — formula strings are preserved, not
  evaluated... will be via integration with an existing Rust formula engine."

So the question is: *which existing engine, and how does it integrate?*

## Position excelrs is in

excelrs today = pure IO, formula **preservation only** (parse `<f>`, write `<f>`,
cached values retained inconsistently). It owns the data model
(`WorkbookInner` / `Worksheet` / `Cell` / `CellValue`) and the FFI boundary
(napi-rs → JS). A formula engine must **drop into** that model, not replace it.

This is the **identical position umya-spreadsheet** (908,848 downloads on
crates.io) occupies. umya's README "Projects using umya-spreadsheet" entry is:

> - **formualizer** — Arrow-backed spreadsheet engine and formula parser with excel
>   parity (xlsx/xlsm via umya-spreadsheet)

i.e. umya (IO) + formualizer (eval). The precedent for the exact pairing we need.

## Decisive criterion: integration shape

excelrs cannot adopt an engine that **owns the workbook model** (that would
replace excelrs). It needs an engine exposing:

  `(formula_string, cell_value_resolver) -> Value`

with the *excelrs model* supplying cell references. Anything workbook-bound is out.

## Candidates (verified via crates.io API + READMEs)

| Crate | Ver | dl | Functions | Dep. graph | Resolver API | Licensed | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **formualizer** / **formualizer-eval** | 0.7.1 | 3,933 (eval) 1,266 (wb) | 400+ | ✓ incremental | **✓ "you own your data model, want just the calculation engine custom resolvers"** | MIT/Apache-2.0 | **Best fit** |
| umya-spreadsheet | 3.0.1 | 908,848 | — (IO only) | — | no eval | MIT | IO sibling, not an engine |
| xlstream-core | 0.4.0 | 309 | 225 | ? (streaming) | whole-file streaming | Apache-2.0 | streaming-only, no single-cell eval |
| recalc-engine | 0.1.0 | 17 | ? | ? | ? | MIT/Apache-2.0 | 17 dl — unusable adoption |
| truecalc-core | 7.0.2 | 1,326 | Google-Sheets semantics | ? | ? | ? | Sheets-not-Excel semantics |
| `formula` (omid) | 0.1.x | low | ~70 | no | bizarre non-standard syntax (`F.MUL(...)`) | — | **Hard out** — README: "does not support parentheses change order of operations", can't even do `1+1` or `SUM(2-1,2)`. Not Excel-compatible. |

## Why formualizer-eval is the clear candidate

1. **Architecture matches**. The crate hierarchy is layered and explicitly allows
   cherry-picking the eval layer alone:

   ```
   formualizer            <- recommended: batteries-included re-export
     formualizer-workbook <- high-level workbook API, sheets, undo/redo, I/O
       formualizer-eval   <- calculation engine, dependency graph, built-ins  ← excelrs wants THIS
         formualizer-parse  <- tokenizer, parser, AST, pretty-printer
         formualizer-common <- shared types (values, errors, references)
   ```

   README "When to use it":
   > `formualizer-eval` — **You own your own data model and want just the
   > calculation engine with custom resolvers**

   This is verbatim the excelrs situation. `formualizer-workbook` (which owns
   sheets/I/O) is an *optional* higher layer; excelrs keeps its own model.

2. **Standalone usage confirmed** — README ships the exact minimal dependency form:

   ```toml
   formualizer-eval = { version = "0.5", default-features = false }
   ```

3. **exceljs parity features present**: dynamic arrays (`FILTER`/`UNIQUE`/`SORT`),
   case-insensitive function names, custom (workbook-local) functions registered
   via `register_custom_function`, callback arg-by-value with range→2D-array
   materialization, array spill. Dependencies tracked incrementally.

4. **Competitor analysis in their own words** (README §"Alternatives"):
   - *calamine* — "read-only — extracts cached values from XLSX cannot evaluate"
     (confirms excelrs's current role).
   - *openpyxl* — no eval engine.
   - *HyperFormula* — closest feature rival, but **AGPL-3.0 (or commercial)** →
     license landmine; formualizer explicitly positions itself as the permissive
     MIT/Apache-2.0 escape valve.
   - *xlcalc* — fraction of function library, partial dep tracking.
   - *formulajs* — ~100 functions, JS-only, no dep graph.

   So formualizer is the only permissive-licensed engine with 400+ functions
   *and* a separate eval subcrate.

### Caveats to validate before committing

- **Adoption lag.** 0.7.1 → downloads in low thousands. It is young (Arrow core,
  recent). Low dl count ≠ unproven (umya already entrusts it for production
  spreadsheet-mcp + excel parity), but it has NOT been battle-tested at excelrs's
  scale. Risk: API churn, edge bugs in <500 functions.
- **License fit:** MIT/Apache-2.0 — compatible with excelrs Dual MIT/Apache-2.0. ✓
- **Dependency weight:** Arrow-powered storage. Pulling in Arrow for a napi-rs
  addon adds compile surface + binary size. `default-features = false` helps but
  must confirm Arrow isn't forced on the eval path.
- **Integration cost:** excelrs must bridge `formualizer-common` values ↔ its own
  `CellValue`, and supply a resolver closure over `WorkbookInner`/cells. Non-trivial
  but bounded — it's a *translator*, not a rewrite.
- **Dynamic arrays** spill into the grid — excelrs model has no concept of
  spill ranges / `#SPILL!` errors. Would need model extension (future scope).

## Other engines — why not

- **xlstream / xlstream-core**: streaming eval (row-by-row, bounded memory).
  Attractive for large-file stories but: 225 functions (not 400+), streaming
  architecture means it can't resolve formulas depending on future rows /
  whole-range refs, and it is **whole-file** (`process_xlsx_to_read`), not
  single-cell eval — can't plug into excelrs's per-WorkbookInner. Lower fit.
- **recalc-engine**: "bug-for-bug Excel recalc" sounds ideal on fidelity, but
  **17 downloads** → effectively zero adoption. Unusable risk for a dependency.
- **truecalc-core**: Google-Sheets semantics, not Excel. excelrs targets ExcelJS
  parity → Excel semantics. Wrong.
- **`formula` (omid)**: rejected for non-standard syntax (see table). Dead end.

## Conclusion

If excelrs ever turns evaluation on (currently a **Non-Goal** with a "not built
scratch" escape hatch), `formualizer-eval` is the single candidate that matches
all constraints:

> permissive license · resolver-shaped API · 400+ Excel functions ·
> dependency graph · layered so the eval crate slots into existing model.

It is also the engine the maintainer of the 908K-download `umya-spreadsheet` IO
library chose for exactly this pairing — a real-world validation of the
integration shape.

The decisive open question is not "which engine" but **"when"**: evaluation is
explicitly deferred and large (500+ functions / dep graph / dynamic arrays /
model bridge). `formularizer-eval` de-risks the "build scratch" fear; the
remaining cost is the translator + model bridge, which ADR-9 already scoped as a
*separate product* decision, not a v2.0.0 capstone item.

> **Update (post-shipped):** `formularizer-eval` was yanked, so the "which
> engine" question resolved itself — excelrs shipped a minimal `bridge.rs`
> evaluator over `xlstream-parse` instead (commit `82b7753`,
> `feat/formula-eval-integration`). This doc's engine recommendation is obsolete;
> the integration-shape analysis above still holds.

## Integration feasibility check (excelrs side)

Before betting on an engine, confirm the **data-model seam exists** — i.e. can a
formula engine bind `(formula_string, cell_ref_resolver) -> Value` against
excelrs's model without a rewrite. **Yes — the seam already exists:**

Resolver inputs are reachable today:

- `src/model/worksheet.rs:155` `get_cell_by_address(&self, address: String) -> Cell`
- `src/model/worksheet.rs:169` `get_cell_by_rc(row, col) -> Cell`
- `src/model/cell.rs:411` `CellValue valueOf` (napi getter `valueOf`) → full
  `CellValue` discriminated union; `number`/`string`/`boolean`/`error_value`
  are the leaf scalars the resolver returns; `formula` + `value_type == "Formula"`
  marks cells the engine must recurse into.

So a resolver closure is ~:

```rust
|addr| worksheet.get_cell_by_address(addr).value_of() {
    if value_type != "Formula" { Some(cached_scalar) } else { None /* recurse */ }
}
```

and `cell.formula()` (`cell.rs:449`, read-only) supplies the formula string.

Model additions needed to *enable* evaluation:

- A Rust method `cell.evaluate()` (or worksheet-scoped recompute) that:
  pulls `formula`, calls `engine.evaluate(formula, resolver)`, writes result
  back into the cached-value fields + emits `<v>` on write
  (`writer/xlsx.rs:1845-1850` already writes a cached number — just needs the
  computed value populated).
- JS mirror: an `#[napi(getter)]` returning the computed scalar (the existing
  `formula()` getter stays read-only; evaluation is a separate accessor to keep
  the model additive and not invent a `cell.formula =` setter that ExcelJS has).
- The engine + dep graph + incremental recalc live **in the new dep**, not in
  excelrs.

Net: **no model rewrite required.** Evaluation is additive plumbing over the
existing read accessors. This is exactly the shape `formualizer-eval` claims
("you own your data model"). Cost = bridge + eval entry points + streaming-path
caveat (see below), not architecture surgery.

### Streaming path caveat

`stream.rs:53` / `build_cell_value` (`stream.rs:778-781`) **deliberately drops**
the cached `<v>` on the streaming path (`has_formula` short-circuits). So the
resolver would return `None` for every formula cell in the streaming reader
→ formulas could not be evaluated from `stream.read()`. Two options if this
matters downstream:

1. Accept the gap (document that stream path is read-only preservation).
2. Let the streaming reader opt *in* to retaining `<v>` (memory-bounded
   tradeoff the design currently avoids on purpose).

This is a **pre-existing asymmetry**, independent of engine choice — worth a
decision note before any eval work.

## Verified: engine resolver contract (the actual API)

Pulled `formularizer-eval`'s real public contract (`src/traits.rs`). The engine is
architected as a **resolver-injection** library — the consumer implements a trait,
the engine owns parse + dep graph + 400+ functions. The excelrs→engine mapping is
one-to-one onto existing accessors:

```rust
// formularizer-eval contract (src/traits.rs)
pub trait ReferenceResolver: Send + Sync {
    fn resolve_cell_reference(sheet: Option<&str>, row: u32, col: u32)
        -> Result<LiteralValue, ExcelError>;   // scalar cell lookup
}
pub trait RangeResolver: Send + Sync {
    fn resolve_range_reference(sheet, sr, sc, er, ec)
        -> Result<Box<dyn Range>, ExcelError>; // row,col grid
}
pub trait NamedRangeResolver: Send + Sync {
    fn resolve_named_range_reference(name: &str)
        -> Result<Vec<Vec<LiteralValue>>, ExcelError>;
}
pub trait TableResolver: Send + Sync {
    fn resolve_table_reference(tref: &TableReference) -> Result<Box<dyn Table>, ExcelError>;
}
pub trait EvaluationContext: Resolver + FunctionProvider + SourceResolver { ... }
```

`Range` itself is a **2-method contract**:

```rust
pub trait Range: Debug + Send + Sync {
    fn get(&self, row: usize, col: usize) -> Result<LiteralValue, ExcelError>;
    fn dimensions(&self) -> (usize, usize);
}
```

Mapping excelrs accessors → engine trait (all already exist on `Worksheet`):

| Engine trait method | excelrs source | effort |
| --- | --- | --- |
| `ReferenceResolver::resolve_cell_reference` | `worksheet.get_cell_by_address(addr)` → `CellValue` cached `number`/`boolean`/`string` | 1:1, trivial |
| `RangeResolver::resolve_range_reference` | wrap `worksheet.get_rows` / cell grid behind `Range` | small adapter |
| `NamedRangeResolver` | `src/model/defined_name.rs` (formula refs in defined names) | low |
| `TableResolver` | `worksheet.get_table` (xlsx only) | low / optional (`NImpl` ok) |
| `formula_text_at_cell` | `cell.formula()` (read-only) | direct |

`EvaluationContext` also demands `FunctionProvider` (builtins — formularizer ships
400+ via `builtins/`, excelrs can delegate or add custom funcs via
`register_custom_function`) and light planks `locale`/`timezone`/`clock`/
`date_system` (1900 vs 1904 — excelrs reads this from workbook; small wire).

**Conclusion flipped from "feasible" to "verified":** the resolver seam is a real,
public, minimal trait. The bridge module is ~1 adapter file + `FunctionProvider`.
The dep is a **translator**, not a rewrite.

### Dependency-weight caveat (real, not hypothetical)

`traits.rs` imports `arrow_array::BooleanArray`, `RangeView`, `rayon::ThreadPool`
at the *eval-context* level. `arrow_store` + `compute_prelude` modules in `lib.rs`
confirm Arrow is core to the eval path (not an opt-out flag). So `formularizer-eval`
pulls **Arrow + arrow_array + rayon** onto the compile graph of this napi-rs addon.
For excelrs this matters: napi-rs native binaries bloat on Arrow + it adds a
heavy build dependency. Must verify `default-features = false` actually drops Arrow
from the eval path before accepting the dep — a non-trivial build-cost question,
not just a license one.

## Sources

- crates.io API metadata: `formualizer` 0.7.1, `formualizer-eval` 0.7.1,
  `umya-spreadsheet` 3.0.1, `xlstream-core` 0.4.0, `recalc-engine` 0.1.0,
  `truecalc-core` 7.0.2, `formula` 0.1.2 (download counts + keywords).
- formualizer README: <https://github.com/psu3d0/formualizer>
- umya-spreadsheet README: <https://github.com/MathNya/umya-spreadsheet>
- excelrs spec: `docs/spec.md` §10 Non-Goals, Appendix A #9 (ADR-9).
