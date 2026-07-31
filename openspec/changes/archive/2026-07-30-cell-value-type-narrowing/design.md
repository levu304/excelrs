## Context

`Cell.value` is the primary read surface for cell content. `v2.5.0` changed the getter so simple cells return the underlying JS primitive (`number`/`string`/`boolean`/`Date`/`null`) while rich cells (Formula, RichText, Hyperlink, Error, Merge) still return the `CellValue` object. A `Cell.type` string-enum accessor was added as the discriminant.

The v2.5.0 proposal and `cell-value-getter/spec.md` asserted this made `CellValue` a *proper discriminated union* enabling narrowing. Two facts contradict that:

1. The generated `CellValue` type is still a flat interface — every variant field is optional on one type. Only `value_type`'s *string* was narrowed to a literal union; the variant *shapes* were not.
2. TypeScript cannot narrow a class getter's return type from a sibling property. `if (cell.type === "Number")` has zero effect on the type of `cell.value`. The spec scenario `if (cell.type === "Number") { const n: number = cell.value; }` is **uncompilable** and was never satisfied.

Empirically, every rich-type read in the test suite still casts: `const result = cell2.value as import('../index').CellValue`. The v2.5.0 change solved the *primitive* papercut but left the *rich-type* papercut fully open.

There is precedent for the cast-free path already in the codebase: `cell.formula`, `cell.note`, and `cell.comment` are dedicated Rust getters returning typed `Option<T>` — no cast, no narrowing needed. RichText is the only rich variant without such an accessor.

## Goals / Non-Goals

**Goals:**

- Make `CellValue` a real discriminated union so `valueType`-based narrowing works in TypeScript.
- Add `cell.valueOf()` returning the full `CellValue` (typed, no cast) for the general "I want the whole thing" case.
- Add `cell.richText` returning `RichTextRun[] | null` for the common "give me the runs" case, mirroring `cell.formula`.
- Fix the impossible narrowing requirement in `cell-value-getter/spec.md`.

**Non-Goals:**

- Changing the `Cell.value` getter's *runtime* behavior. Simple cells still return primitives; rich cells still return the `CellValue` object. Only the *type* of `CellValue` changes.
- Changing the Rust `CellValue` struct or its serialization. The discriminated union is a `.d.ts` generation concern only.
- Adding accessors for Formula/Hyperlink/Error (`cell.formula` already exists; the others are lower-frequency — defer to a later change if demanded).
- Removing `cell.date` or `cell.type` (back-compat retained).

## Decisions

### D1 — Discriminated union is a `.d.ts` transform, not a Rust change

**Decision:** Keep the Rust `#[napi(object)] pub struct CellValue` exactly as-is. In `scripts/apply-glue.cjs`, extend the DTS pass to replace the generated `export interface CellValue { … }` block with:

```typescript
export type CellValue =
  | { valueType: "Null" }
  | { valueType: "Number"; number: number }
  | { valueType: "String"; string: string }
  | { valueType: "Boolean"; boolean: boolean }
  | { valueType: "Date"; dateSerial: number }
  | { valueType: "Formula"; formula: string }
  | { valueType: "Error"; errorValue: string }
  | { valueType: "Hyperlink"; hyperlink: string; hyperlinkText?: string }
  | { valueType: "RichText"; richText: RichTextRun[] }
  | { valueType: "Merge" };
```

**Rationale:** The runtime object already serializes to one of these shapes (napi-rs emits all `Option` fields as `null` when absent; excess `null` properties are permitted on a non-literal value). Changing the Rust struct to a real enum is impossible (napi-rs v3 cannot cross FFI with data-carrying variants — the original architecture-review P0). A post-build transform is the same injection point already used for `CellValueResult` / `Cell` merge in `dts-header.d.ts`, so it is consistent with the existing pipeline.

**Alternatives considered:**

- *Mark the struct `#[napi(object, ts_type = "CellValueUnion")]` and define `CellValueUnion` in the header.* Rejected: the generated code would reference `CellValueUnion`, but `dts-header.d.ts` and all tests reference `CellValue` — a name mismatch requiring an alias and confusing two names for one type.
- *Hand-edit `index.d.ts` after build.* Rejected: `napi build` regenerates it; not sustainable (same reason `dtsHeaderFile` exists).

### D2 — `cell.valueOf()` returns the full discriminated union

**Decision:** Add a Rust method (not a getter, to avoid colliding with the `value` getter and JS `valueOf` semantics confusion is acceptable — it is a method call):

```rust
#[napi]
pub fn value_of(&self) -> CellValue {
    self.inner.lock().expect("Cell lock poisoned").value.clone()
}
```

**Rationale:** Gives a single, always-typed path to the whole `CellValue` without the primitive-unwrapping that `cell.value` does. Users who want type-safe rich-type access write `const cv = cell.valueOf(); if (cv.valueType === "RichText") cv.richText;`. Naming `valueOf` intentionally matches the conceptual "give me the value object" and is distinct from the `value` getter (which unwraps). The existing internal `value_raw()` already does this clone; this just exposes it publicly with the corrected TS type.

**Alternatives considered:**

- *Overload the `value` getter.* Rejected: napi-rs cannot produce overloaded getter return types, and changing `cell.value` to always return `CellValue` would regress the v2.5.0 primitive-unwrapping DX win.
- *Type guard functions `isRichText(cell)`.* Rejected: TS cannot narrow a class getter's return type via an `is` guard on the instance — the same limitation that makes the spec scenario impossible.

### D3 — `cell.richText` dedicated getter mirrors `cell.formula`

**Decision:** Add a Rust getter:

```rust
#[napi(getter)]
pub fn rich_text(&self) -> Option<Vec<RichTextRun>> {
    let inner = self.inner.lock().expect("Cell lock poisoned");
    inner.value.rich_text.clone()
}
```

**Rationale:** RichText is the highest-frequency rich variant after Formula (which already has `cell.formula`). It is the only rich variant without a dedicated accessor, so rich-text reads are the ones that most often require the `as CellValue` cast today. Mirroring `cell.formula` / `cell.note` / `cell.comment` keeps the API consistent and gives a zero-cast path: `if (cell.type === "RichText") { cell.richText }`.

**Alternatives considered:**

- *Add `cell.hyperlink` and `cell.errorValue` getters too.* Deferred: lower frequency; this change should stay scoped to the variant the user explicitly called out (RichText). Can be added later without breaking anything.
- *Only do D2 (`valueOf`) and skip the dedicated getter.* Rejected: `cell.richText` is strictly more ergonomic for the 80% case and matches an existing pattern the codebase already committed to.

### D4 — Fix the spec's impossible narrowing requirement

**Decision:** In `cell-value-getter/spec.md`, replace the "Narrowing works in TypeScript" requirement (which asserts `cell.type`-based narrowing) with two achievable requirements:

1. `CellValue` SHALL be a discriminated union narrowable on `valueType` (via `cell.valueOf()`).
2. `cell.richText` SHALL return `RichTextRun[] | null` without a cast when the cell is a RichText cell.

**Rationale:** The original requirement described behavior TypeScript cannot express for class getters. Leaving it would keep a spec that can never be green. The corrected requirements describe what this change actually delivers.

## Risks / Trade-offs

- **[TS-only breaking shape change]** Downstream code that destructures the flat `CellValue` (e.g. `const { number, string } = cell.value as CellValue` on a Number cell) may need adjustment, because `number` is now required-only on the `Number` branch. → Mitigation: this is a type-only change; runtime objects are identical. Most consumers used `cell.value` (primitive) or `as CellValue` already. Ship as patch `v2.5.1`; document in changelog.
- **[`Partial<CellValue>` setter input]** With `CellValue` now a union, `Partial<CellValue>` becomes a union of partials. `cell.value = { richText: [...] }` still type-checks (matches the partial RichText branch). Bonus: TS now *rejects* invalid combos like `{ valueType: "RichText", number: 42 }` at compile time (excess-property error) — strictly better than today.
- **[dts transform fragility]** Replacing the `CellValue` interface via regex/string in `apply-glue.cjs` depends on the generated shape. → Mitigation: the generated `CellValue` block is stable (struct unchanged); anchor the replacement on the `export interface CellValue` start marker and the matching closing brace. Add a unit test in the glue script or a CI `typecheck` step that fails if `CellValue` is not a union.
- **[`valueOf` naming]** JS objects have a `valueOf()` method (returns primitive). Ours returns an object. → Mitigation: document the deviation; it is a method, not overriding the prototype `valueOf`. Acceptable and clearly named.

## Migration Plan

1. Implement `value_of()` + `rich_text()` getters in `src/model/cell.rs`.
2. Extend `scripts/apply-glue.cjs` DTS pass to replace the flat `CellValue` interface with the discriminated union `type CellValue = …`.
3. Rebuild (`pnpm build`); verify `native.d.ts` / `index.d.ts` regenerate with the union and the two new members.
4. Update `__test__/rich-text.test.ts` to drop `as CellValue` casts (use `cell.richText` / `cell.valueOf()`). Add a narrowing compile-check test.
5. Update `__test__/cell.test.ts` with `valueOf` / `richText` assertions.
6. Fix `cell-value-getter/spec.md` + add `rich-text/spec.md` requirement. Bump version (`v2.5.1`) + CHANGELOG note.
7. Rollback: revert the `apply-glue.cjs` DTS transform + the two Rust getters; `CellValue` returns to flat interface.
