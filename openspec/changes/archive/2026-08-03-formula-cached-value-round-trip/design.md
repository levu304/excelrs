# Design: Authorable cached formula values (round-trip)

See `proposal.md` for motivation. This change is the minimal, engine-independent slice of
issue #54: carry a **caller/Excel-supplied** cached scalar through `Cell.value = { formula, … }`
→ `<f><v>` → read-back as `cell.value` (scalar) + `cell.formula`.

## Context

excelrs IO layer: calamine two-pass read + `zip`/`quick-xml` write; `WorkbookStream` SAX streaming.
Formula strings are opaque `<f>`; cached scalars live in `CellValue` fields on the `Formula`
variant. The write arm (`writer/xlsx.rs:1855`, `"Formula" =>`) already serializes every scalar
field as `<v>`; the `cached_value` getter (`cell.rs:458`) already reads them. The only missing
link is the **setter** (`cell.rs:388`) branch-3, which constructs the `Formula` `CellValue`
and discards any supplied scalar:

```rust
} else if let Some(f) = obj.get("formula").and_then(|v| v.as_str()) {
    CellValue::formula(f.to_string())          // drops number/string/boolean/error_value/date_serial
}
```

Branch-4 (`valueType` key) already reads those same fields, so the model already *supports*
carrying them — branch-3 just never reaches that code.

## Goals / Non-Goals

**Goals:**

- Let the JS/TS author set a cached scalar on a formula cell and have it survive the round-trip
  as `cell.value`.
- Zero new `CellValue` fields / variants; reuse existing scalar fields the writer and getter
  already consume.
- Add fixture coverage for the cached-value **read path** (currently unguarded).

**Non-Goals:**

- In-process formula evaluation (engine-independent).
- Changing `Cell.cachedValue` getter semantics (D1a — recalc-only, see Decisions §1).
- Matching ExcelJS's `{ formula, result }` object form for `cell.value` (pre-existing divergence,
  see Risks §4).
- Streaming reader/writer (`stream.rs`) changes.

## Decisions

### 1. `cachedValue` getter stays recalc-only (D1a — `null` on disk/authoring)

`cached_value` (`cell.rs:458`) returns `None` when `value_type != "Formula"`. On disk-read — and
on read-back of an authored cache — calamine re-tunes the cell to a scalar `value_type` (e.g.
`Number`) via `map_data`, so `cachedValue` is `null` even though `cell.value` carries the cached
scalar. This is **current v2.8.0 behavior** and issue #54 explicitly scopes itself to the
authoring round-trip + a fixture, **not** a re-definition of `cachedValue`.

- **Alternative:** relax the guard to also surface Excel-authored caches
  (`formula.is_some() && value_type ∈ {Number, String, Boolean, Error, Date}`). **Rejected for
  this slice**: it changes `cachedValue`'s return value for *every* real-Excel formula cell on
  disk-read — a behavior change, riskier, and exactly the scope creep issue #54 avoids. Tracked
  as an Open Question (§Open Questions) for a future ADR.

### 2. Reuse scalar fields as the authoring surface (D2a — zero new fields)

Do **not** add a dedicated `cachedValue` input key. Fold `number`/`string`/`boolean`/
`error_value`/`date_serial` from the input object onto the `Formula` `CellValue` in branch-3:

```rust
} else if let Some(f) = obj.get("formula").and_then(|v| v.as_str()) {
    let mut cv = CellValue::formula(f.to_string());
    cv.number      = obj.get("number").and_then(|v| v.as_f64());
    cv.string      = obj.get("string").and_then(|v| v.as_str()).map(str::to_string);
    cv.boolean     = obj.get("boolean").and_then(|v| v.as_bool());
    cv.error_value = obj.get("errorValue").and_then(|v| v.as_str()).map(str::to_string);
    cv.date_serial = obj.get("dateSerial").and_then(|v| v.as_f64());
    cv
}
```

- The writer `"Formula"` arm already emits `<v>` for `number`, `string`, `boolean`,
  `error_value` (`writer/xlsx.rs:1855`). It does **not** yet emit `<v>` for a cached `date_serial`
  on the Formula arm (the `Date` arm at `xlsx.rs:1916` handles `date_serial`, but Formula cells
  never match it) — this change **adds that branch**, mirroring the `Date` arm:
  `if let Some(serial) = cv.date_serial { write_str(w, &format!("<v>{}</v>", serial))?; }`.
- the `cached_value` getter already reads them (including `date_serial`, `cell.rs:480`). So
  authoring + write + (recalc) read all flow with **no other Rust edits**.
- TS surface widens the `Formula` arm of the `CellValue`/`CellValueInput` union to carry the
  optional scalar fields (`number?`, `string?`, `boolean?`, `errorValue?`, `dateSerial?`).

- **Alternative:** D2b, a dedicated `cachedValue` input key. **Rejected:** would need its own
  coercion path in `set_value` (a new branch) and maps awkwardly to the existing scalar fields;
  the field-reuse form is the smaller diff and matches the in-memory `CellValue` shape already.

### 3. Fixtures — ExcelJS `result` authoring + a hand-crafted read-path xlsx (D3)

Two fixtures, both asserting `cell.value` + `cell.formula` (never `cachedValue`, per D1a):

1. **ExcelJS-authored** (isolated read coverage is *not* this; this covers the full authorship
   round-trip): `cell.value = { formula: "A2+B2", result: 3 }` — ExcelJS emits
   `<f>A2+B2</f><v>3</v>` (`node_modules/exceljs/lib/xlsx/xform/sheet/cell-xform.js:158-189`,
   dispatches on `model.result`). This also debunks issue #54's stale assumption that "ExcelJS
   never writes cached `<v>`"; the existing `reader.test.ts` formula tests just omitted `result`.
2. **Hand-crafted `.xlsx`** carrying `<f>..</f><v>..</v>` directly — locks the pure read path
   independent of the writer/authorship code, so a regression in `map_data` /
   `worksheet_range` / `worksheet_formula` (not the setter) is a hard failure.

Both committed under `openspec/.../fixtures/` and copied into `__test__/fixtures/` for the TS
suite.

## Implementation sketch

- `src/model/cell.rs` `set_value` branch-3: fold scalar fields (snippet §2).
- `index.d.ts`: `Formula` arm gains optional `number?`, `string?`, `boolean?`, `errorValue?`,
  `dateSerial?` on both `CellValue` (getter) and the input `CellValueInput` union.
- `__test__/`: add `cached-formula.test.ts` (ExcelJS round-trip + read-back of the committed
  xlsx), asserting typed `cell.value` and `cell.formula`.
- `openspec/.../fixtures/`: `exceljs-cached-formula.xlsx` (generated) + `hand-cached-formula.xlsx`
  (hand-crafted, committed for read-path isolation).

## Risks / Trade-offs

- **In-memory vs. read-back `value_type` duality.** Authoring sets the cell to `value_type =
  "Formula"` (in-memory); after write+read-back calamine re-tunes it to the cached scalar's type
  (e.g. `Number`). So `cell.value` for a formula cell is the **bare scalar** (`3`), not ExcelJS's
  `{ formula, result }` object. This diverges from ExcelJS's `cell.value` contract for formula
  cells.
  - *Decision:* preserve v2.8.0 behavior (issue #54 is scoped to authoring round-trip + fixture,
    not formula-cell `value` semantics). Do not align here; flag for a future ADR.
  - *Mitigation:* fixtures assert the excelrs contract (scalar `cell.value`), documented in spec
    §R5 / Non-Requirement.
- **`cachedValue` stays `null` for authored/Excel caches (D1a).** A consumer reading a disk
  formula with `<v>` gets the cached value via `cell.value` but `cell.cachedValue` is still
  `null` unless they ran `recalculate()`. Documented gap (spec §R4), not a regression — it is
  current behavior and explicitly out of scope.
- **TS type-sync:** native.d.ts is hand-glue'd via `apply-glue.cjs` (never auto-regenerated,
  which is how `cachedValue` first went unshipped for TS). Widening here must be applied in
  *both* `index.d.ts` and `native.d.ts` manually. Add a task that lints for parity.

## Blast radius (GitNexus impact)

- `Cell::set_value` `cell.rs:388` — edited (branch-3 fold). Direct callers: napi `set_value`
  binding, `Cell` JS setter. No signature change.
- `cached_value` getter `cell.rs:458` — **read-only** (not edited).
- `Cell::formula` setter path / `CellValue::formula` — read-only.
- `writer/xlsx.rs:1855` `"Formula"` arm — **edited** (additive `date_serial` branch mirroring the
  `Date` arm at `xlsx.rs:1916`). No change to the existing number/string/boolean/error `<v>` emission.
- Rust `set_cached_value_raw` (`cell.rs:596`, recalc path) — **read-only**.

Net: the change touches setter branch-3 + the writer `date_serial` branch + the TS union
- new tests/fixtures. Low risk; all edits additive, no signature changes.

## Migration Plan

- **Deploy:** additive. Existing consumers authoring `{ formula }` without a cached scalar are
  unaffected (no scalar fields present → `None`/default; behavior unchanged — scenario
  "formula authored without a cached value still reads back").
- **Rollback:** revert branch-3 fold + TS union widening; no persisted format change.
- **Docs:** note authorable cached-result support in `docs/spec.md` §10 and a `CHANGELOG.md`
  entry under the formula-cache behavior.

## Open Questions (deferred)

- **OQ1.** Should `cachedValue` getter be unified (D1b) to surface Excel-authored / authored
  caches, aligning the getter with ExcelJS's `result` semantics? Deferred — scope for a separate
  ADR + behavior-change review. This change deliberately keeps D1a.
- **OQ2.** Should excelrs `cell.value` for formula cells align to ExcelJS's `{ formula, result }`
  object form (divergence documented §Risks-1)? Out of scope; pre-existing.
- **OQ3.** Does `apply-glue.cjs` need a parity-lint task so hand-written `native.d.ts` cannot drift
  from `index.d.ts` again (the root cause of `cachedValue` being unshipped for TS)? Proposed as a
  follow-up infra task (see tasks.md).
