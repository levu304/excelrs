# Tasks: Formula Evaluation (formularizer-eval)

Ordered by dependency. Each task verifiable against `specs/formula-eval/spec.md`
(see `design.md` for how). The design's Open Question #1 (stateless per-call) is resolved
here: **stateless per-cell evaluation first** (task 3.1). Open Question #2 (Arrow dropped
via `default-features=false`) resolved by task 1.2.

> ⚠️ **BLOCKER**: `formularizer-eval` v0.7.1 yanked from crates.io (API 404, sparse
> index NoSuchKey, GitHub `psu3d0/formularizer` 404). Dependency commented out in
> `Cargo.toml`, `formula-eval` feature is no-op. Default build unblocked. Tasks 2.1–2.4,
> 3.1–3.3, 5.1–5.5 blocked pending decision — see `docs/formula-engine-research.md` §Alternatives.

## 1. Dependency & build setup

- [x] 1.1 Add `formularizer-eval` dependency + `formula-eval` Cargo feature to `Cargo.toml`
      ⚠️ crate subsequently yanked; dependency commented out, feature is no-op.
      (off by default; default features must not pull it in).
- [x] 1.2 Validate build weight: confirm `default-features = false` keeps Arrow/rayon off the
      default graph and that enabling `formula-eval` does not break the default build; record
      binary-size baseline (default vs `formula-eval`).

## 2. Evaluation bridge

- [💀 BLOCKED] 2.1 Create `src/formula/mod.rs` (gated by `#[cfg(feature = "formula-eval")]`)
      implementing `formularizer_eval::EvaluationContext` over excelrs read accessors.
- [💀 BLOCKED] 2.2 Implement `ReferenceResolver::resolve_cell_reference` → `Worksheet::get_cell_by_address`
      → `CellValue` leaf scalar (number/boolean/string/error).
- [💀 BLOCKED] 2.3 Implement `RangeResolver` by wrapping worksheet rows behind the engine's
      `Range` trait (`get` + `dimensions`).
- [💀 BLOCKED] 2.4 Wire `NamedRangeResolver` / `TableResolver` (NImpl for unsupported first phase).

## 3. Entry points & JS API

- [💀 BLOCKED] 3.1 Add Rust `Cell::evaluate(&self) -> Result<Scalar, ExcelError>` and
      `Worksheet::recalculate` delegating to `formularizer_eval` through the bridge.
- [💀 BLOCKED] 3.2 Add JS `Cell.cachedValue` napi read-only getter returning the computed scalar
      or `null` (spec §1, §4).
- [💀 BLOCKED] 3.3 Confirm `Cell.formula` remains a read-only getter (no setter) — ExcelJS parity.

## 4. Cached value & write path

- [ ] 4.1 Store evaluated scalar on `CellValue` cached fields so the writer emits
      `<f>{formula}</f><v>{computed}</v>` for evaluated cells
      (`writer/xlsx.rs:1845-1850`).
- [ ] 4.2 Streaming-path limitation guard: streaming reader keeps formula strings only;
      `cachedValue` absent on streaming cells (spec §5).

## 5. Testing

- [💀 BLOCKED] 5.1 Test per-cell evaluation over numeric cells returns computed scalar (spec scenario).
- [💀 BLOCKED] 5.2 Test a referenced `#DIV/0!` returns an error sentinel, not a panic.
- [💀 BLOCKED] 5.3 Test cross-sheet reference (`Sheet2!A1`) resolves by name.
- [💀 BLOCKED] 5.4 Test written xlsx contains `<f>…</f><v>{computed}</v>` for an evaluated cell.
- [💀 BLOCKED] 5.5 Build/test without `formula-eval`: evaluation API is absent from public surface.

## 6. Specs & docs

- [ ] 6.1 Supersede Non-Goal text in `docs/spec.md` §10 (evaluation now in-scope via feature).
- [ ] 6.2 Update `ROADMAP.md` formula-evaluation status (gated behind `formula-eval`).
- [ ] 6.3 Add CHANGELOG entry keyed to the `formula-eval` feature.
