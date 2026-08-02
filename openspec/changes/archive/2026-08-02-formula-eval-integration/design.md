# Design: Formula Evaluation (formularizer-eval)

See `proposal.md` for motivation. This document covers how evaluation is wired in.

## Context

excelrs is a `Workbook(Arc<Mutex<WorkbookInner>>)` IO layer (calamine read / zip+quick-xml write,
plus `WorkbookStream` SAX streaming). Formula strings are preserved as opaque `<f>` payloads; the
only model seam is read accessors on `Worksheet` (`get_cell_by_address`, `worksheet.rs:155`) and
`Cell` (`formula()` read-only getter `cell.rs:449`; `CellValue` leaf scalars `cell.rs:411`).

`formularizer-eval` (0.7.x, MIT/Apache-2.0) is architected as a *resolver-injection* engine: the
consumer implements `EvaluationContext` (extending `ReferenceResolver`, `RangeResolver`,
`NamedRangeResolver`, `TableResolver`) and the engine owns parse + 400+ builtins + dependency
graph. `Range` is a 2-method `get`+`dimensions` contract. ExcelJS parity is preserved:
`cell.formula` stays read-only; a separate `cell.cachedValue` read accessor is added.

See `specs/formula-eval/spec.md` for the behavior contract this design satisfies.

## Goals / Non-Goals

**Goals:**

- Add evaluation as a drop-in, opt-in feature with zero mutation of `WorkbookInner`/`Cell`
  internals; engine receives resolvers, not the model.
- Reuse the existing read accessors as the resolver; emit cached `<v>` via the writer path that
  already handles it (`writer/xlsx.rs:1845-1850`).
- Keep the JS surface ExcelJS-aligned: read-only `formula`, additive `cachedValue`.

**Non-Goals:**

- Dynamic-array spill ranges / `#SPILL!` propagation.
- Whole-workbook, dependency-ordered recalc pass (per-cell evaluation only, first phase).
- Streaming-path evaluation (cached `<v>` is still dropped on the streaming reader; spec §5
  documents this as a gap).
- Replacing the formula-preservation behavior on the default build.

## Decisions

### 1. Engine: `formularizer-eval`

Chosen over `xlstream-core` (whole-file, 225 funcs), `recalc-engine` (17 downloads),
`truecalc-core` (Google-Sheets, not Excel), `formula`/omid (no parens support). Rationale:
resolver-injection API maps 1:1 onto existing accessors; 400+ Excel functions; permissive
license (compatible with Dual MIT/Apache-2.0); real-world precedent as the eval layer paired
with the IO layer `umya-spreadsheet` (908K dl).

- **Alternative considered:** build a parser/eval from scratch — rejected by ADR-9 ("separate,
  massive undertaking … Excel supports 500+ functions"); integration is the mandated escape hatch.
- **Alternative considered:** HyperFormula — rejected: AGPL-3.0, license-incompatible.

### 2. Cargo feature `formula-eval` (off by default)

Gates the dep and the API so the default crate graph is unchanged.

```toml
[features]
formula-eval = ["dep:formularizer-eval"]
```

- **Alternative considered:** always-on dep — rejected: Arrow/rayon weight on every consumer build
  (see Risks).

### 3. Additive evaluator, model untouched

Bridge module `src/formula/mod.rs` implements `EvaluationContext` by delegating:
`ReferenceResolver::resolve_cell_reference` → `worksheet.get_cell_by_address` → `CellValue`
leaf scalar; `RangeResolver::resolve_range_reference` → wrap the worksheet row grid behind the
`Range` trait; `formula_text_at_cell` → `cell.formula()`. No `WorkbookInner`/`Cell` field changes.

- **Alternative considered:** store a per-workbook engine/interpreter in `WorkbookInner` —
  rejected: would mutate the shared model under the `Arc<Mutex<_>>` lock and force evaluation
  state into every build.

### 4. Entry points & write path

Rust: `Cell::evaluate(&self) -> Result<LiteralValue, ExcelError>` and
`Worksheet::recalculate(...)`. JS mirror: `Cell.cachedValue` napi getter (read-only). On write,
the computed scalar flows into the existing `<v>` emitter (`writer/xlsx.rs`).

- **Alternative considered:** eager evaluation on cell mutation — rejected: breaks streaming
  constant-memory invariant; deferred to an explicit `recalculate` call.

### 5. JS API shape

`cell.formula` remains a read-only getter (no setter, ExcelJS parity). `cell.cachedValue` is a new
read-only getter returning the computed scalar or `null`. No setter introduced — the model is IO,
not a mutable spreadsheet engine.

## Risks / Trade-offs

- **[Arrow / rayon build weight on enabled path]** → Mitigation: feature-gated off by default;
  `default-features = false` validated before release (build-weight validation task). Napi-rs
  binary stays slim for non-eval consumers.
- **[Engine immaturity]** → Mitigation: `formularizer-eval` is 0.7.x with young download counts;
  mitigated by umya-spreadsheet adoption, but treat first 400-function surface as beta; add error
  sentinel (not panic) on any unsupported expression. Tracked as gap in spec §3.
- **[Spill ranges / `#SPILL!` not modeled]** → Mitigation: explicitly out of scope (Non-Goals);
  bridge is layered so a future spill resolver can slot in without touching entry points.
- **[Streaming path cannot evaluate]** → Mitigation: documented limitation (spec §5); streaming
  reader keeps formula strings only. No regression vs. current Non-Goal.
- **[ABI/API churn from upstream trait changes]** → Mitigation: bridge module isolates the
  `EvaluationContext` impl behind a single internal adapter so upstream API drift is one-file
  surface, not a model rewrite.

## Migration Plan

- **Deploy:** opt-in. Existing consumers unaffected (default features exclude `formula-eval`).
  Early adopters enable feature + call `recalculate()`/`cachedValue`.
- **Rollback:** disable feature — API disappears, `<f>` still written as today, no behavior loss.
  No persisted state introduced.
- **Docs:** update `docs/spec.md` §10 Non-Goal and `ROADMAP.md` to reflect eval is now
  in-scope-via-integration behind the feature.

## Open Questions

- Should the bridge expose a `Worksheet::recalculate` that caches results in-cell, or remain
  stateless per `Cell::evaluate` call? (Resolved in tasks: stateless per-call first; caching is
  a follow-up.) — kept per spec §3; not a blocker.
- Does pinning `formularizer-eval` require the `default-features=false` flag to drop Arrow from
  the eval-context path? (Validation task; answer feeds Cargo feature definition.)
