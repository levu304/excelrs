## Context

`Cell.value` is the primary read/write surface for cell content. Today the getter returns the Rust `CellValue` struct verbatim — a flat `#[napi(object)]` with a `valueType: string` discriminant and optional typed fields (`number`, `string`, `boolean`, …). This is the napi-rs FFI-safe shape, but it leaks the Rust representation into TypeScript: a number cell yields `{ valueType: "Number", number: 42 }` instead of `42`.

ExcelJS — the library excelrs ports — returns the actual JS primitive from `cell.value` and exposes a separate `cell.type` discriminant (`ValueType` enum). excelrs already has most of the pieces: a working setter that accepts primitives, a `cell.date` getter returning `Date`, and `napi-rs`'s `to_unknown()` / `ts_return_type` mechanisms. Only the getter path and the TS contract need to change.

This change is the combined "Phase A + Phase B" the exploration settled on, shipped together in `v2.5.0`.

## Goals / Non-Goals

**Goals:**

- `Cell.value` getter returns the underlying primitive for Number/String/Boolean/Date/Null cells.
- Add `Cell.type` string-enum accessor as the discriminant replacement for `cell.value.valueType`.
- Make the TypeScript `CellValue` a proper discriminated union (`CellValueResult`) so reads narrow correctly.
- Zero new Rust/JS dependencies; reuse `to_unknown()` and `dtsHeaderFile`.

**Non-Goals:**

- Changing the setter dispatch (already accepts primitives + `CellValue`).
- Changing `Row.values` / `Row.addRow` (already `Array<any>`).
- Changing `TableRow.values` (table-specific data shape, stays `Array<CellValue>`).
- Returning primitives for rich cells (Formula/RichText/Hyperlink/Error/Merge keep `CellValue` shape).
- Migrating `cell.date` removal — kept for back-compat, deprecation noted for v3.

## Decisions

### D1 — Getter returns `Unknown`, converts per-variant

**Decision:** The getter signature becomes `pub fn value(&self, env: Env) -> napi::Result<Unknown<'_>>` and matches on `value_type`, building the right JS value with `Env` helpers.

**Rationale:** napi-rs cannot express "a function that returns different Rust types" cleanly. Returning `JsUnknown` (or `Any`) is the supported escape hatch; `ts_return_type` overrides the generated `.d.ts` so the TS contract stays precise.

**Per-variant conversion:**

- `Number` → `env.to_js_value(&cv.number.unwrap_or(f64::NAN))`
- `String` → `env.to_js_value(&cv.string.as_deref().unwrap_or(""))`
- `Boolean` → `env.to_js_value(&cv.boolean.unwrap_or(false))`
- `Date` → `env.create_date(ms)` then `JsDate::to_unknown()` (Path A — same `transmute` trick already in the `date` getter at line 255)
- `Null` → `env.get_null().to_unknown()`
- `_` (Formula/RichText/Hyperlink/Error/Merge) → `env.to_js_value(cv)` (serde-serialized `CellValue`)

**Alternatives considered:**

- *`napi::Either` up to `Either26`* — would require nesting 10+ variants and still needs `ts_return_type` overrides; rejected as more code, no gain.
- *Keep Date as `CellValue`* — simpler but leaves a visible inconsistency vs. the other 4 simple types; rejected now that `to_unknown()` is confirmed available.

### D2 — `cell.type` as a `string_enum` Rust enum

**Decision:** Add `#[napi(string_enum)] pub enum CellType { Null, Number, String, Boolean, Date, Formula, Error, Hyperlink, RichText, Merge }` and a `#[napi(getter, js_name = "type")]` `value_type()` method returning `CellType`.

**Rationale:** String enums generate a TypeScript `enum` whose members are string literals (`CellType.Number === "Number"`), which makes `cell.type === "Number"` readable and keeps the discriminated union usable without importing extra constants. ExcelJS uses a numeric `ValueType` enum; string literals are better DX and we are not bound to mirror ExcelJS internals.

**Alternatives considered:**

- *Return `String` from getter + `ts_return_type` literal union* — works, but a `string_enum` is less boilerplate and guarantees the union members match the Rust variants.

### D3 — TypeScript union via `dtsHeaderFile`

**Decision:** Add `"dtsHeaderFile": "./dts-header.d.ts"` to the `napi` config in `package.json`. The header declares `CellValueResult`, `CellSimpleValue`, `CellType` (re-export), and a `Cell` interface declaration merge overriding `get value()` and `get type()`.

**Rationale:** The auto-generated `index.d.ts` (built from `native.d.ts` + `scripts/apply-glue.cjs`) cannot be hand-edited sustainably — `napi build` regenerates it. The header is the supported injection point for custom types and merges, and it sits at the top of the file so the union type is in scope before it is referenced. The existing `Cell` setter merge (currently appended by `apply-glue.cjs`) moves into the header to keep all `Cell` type overrides in one place.

**Open question (resolved):** Should `cell.value` for Date return a `Date` or keep `CellValue`? → Returns `Date` (see D1).

### D4 — `cell.date` retained, deprecated for v3

**Decision:** Keep the existing `get date()` getter returning `Date | null`. Add a deprecation note in the TS docs pointing users to `cell.value` (which now returns `Date` for Date cells).

**Rationale:** Removing it would be a second breaking change with no functional benefit; keeping it costs one redundant getter. Mark deprecated so v3 can drop it cleanly.

## Risks / Trade-offs

- **[Breaking change in a minor version]** v2.5.0 ships a behavior break (`cell.value` no longer returns `CellValue` for simple cells). → Mitigation: explicit release-notes entry; `cell.type` + rich-type `CellValue` keep the old shape available; offer a codemod snippet in the changelog.
- **[`cell.value` loses `.valueType` on primitives]** Code that read `cell.value.valueType` on Number/String/Boolean/Null breaks. → Mitigation: `cell.type` is the replacement; tests updated; migration guide provided.
- **[`to_unknown()` lifetime]** JsDate must be extended to `'static` via `transmute` (same pattern as the `date` getter). → Mitigation: the underlying `napi_value` is valid for the environment's lifetime and consumed immediately by the generated wrapper; identical to existing code.
- **[`f64::NAN` for missing Number]** A Number cell with `None` serializes to `NaN`. → Mitigation: internal model always populates `number` on store; `NaN` only appears on corrupt state, acceptable.

## Migration Plan

1. Implement Rust getter + `CellType` enum + `value_type()` getter.
2. Add `dts-header.d.ts`; point `package.json` `napi.dtsHeaderFile` at it; move the `Cell` setter merge out of `apply-glue.cjs` into the header.
3. Rebuild (`pnpm build`); verify `native.d.ts`/`index.d.ts` regenerate with the new union + `type` getter.
4. Update tests: `__test__/cell.test.ts`, `__test__/xlsx-async-contract.test.ts` (simple-type assertions → `cell.type` / `cell.value`); rich-text tests unchanged.
5. Bump `package.json` + `Cargo.toml` to `2.5.0`; add CHANGELOG entry.
6. Rollback: revert the change branch; no schema/data migration involved, the `.node` binary is the only artifact.

## Open Questions

- Should `cell.date` be formally `@deprecated` in the `.d.ts` (TS 5.x JSDoc `@deprecated` tag) in this release, or only noted in docs? → Recommend `@deprecated` tag now, removal in v3.
- Any downstream consumers rely on `cell.value` returning `CellValue` for Number cells (e.g., `Array.isArray(cell.value)` checks)? → Add migration note covering `typeof`/`cell.type` discrimination.
