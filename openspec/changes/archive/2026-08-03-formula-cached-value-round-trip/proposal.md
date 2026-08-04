# Proposal: Authorable cached formula values (round-trip)

## Why

excelrs preserves formula strings as opaque `<f>` payloads and — since v2.7.0 via the
`formula-eval` integration attempt — also exposes a `cachedValue` getter. But `cachedValue`
is recalc-only, and **formula cells authored through the JS setter cannot carry a cached
result across the write→read round-trip** (issue #54). The read path *does* work for
Excel-authored files that contain `<f>..</f><v>..</v>`, but **there is no committed fixture**
that exercises it, so the only regression guard for the cached-value read path is a
`rich-text.test.ts` assertion on `formula` that never checks `value`/`cachedValue`.

This change closes the gap with the smallest possible diff: let the setter fold a supplied
cached scalar into the `Formula` `CellValue`, let the writer emit `<v>` for it (the `"Formula"`
arm already emits `number`/`string`/`boolean`/`error_value`; this change **adds the missing
`date_serial` branch**), and add a fixture asserting the read path returns the cached scalar

+ formula string.

> Note on context: the earlier in-repo `formula-eval` change attempted to integrate an
> external evaluation engine (`formularizer-eval`). That crate was yanked from crates.io
> (404 on the GitHub org and sparse index), so that change is **blocked / deferred**. This
> change is independent of any eval engine: it carries **user/Excel-supplied** cached
> results, not computed ones.

## What changed (already true in v2.8.0, stale in issue #54)

Issue #54 was filed against v2.6.0 and states "the writer only honors `cv.number`." That is
no longer true:

+ `src/writer/xlsx.rs:1855` (`"Formula" =>` arm) already emits `<v>` for `cv.number`,
  `cv.string` (shared-string index), `cv.boolean`, and `cv.error_value`. It does **not**
  yet emit `<v>` for a cached `date_serial` (date-serial cached values are dropped on the
  Formula arm; the `Date` arm at `xlsx.rs:1916` handles them, but Formula cells never reach it).
+ `src/model/cell.rs:458` `cached_value` getter reads those scalar fields back for a
  `Formula`-typed cell (including `date_serial`).

So the write arm is **near-complete**: missing piece #1 is getting cached scalars *into* the
`Formula` `CellValue` at authoring time (setter), and #2 is emitting `<v>` for a cached
`date_serial` on the Formula arm — both covered here.

## How small

`set_value` (`src/model/cell.rs:388`) branch-3 currently short-circuits on the `formula` key:

```rust
} else if let Some(f) = obj.get("formula").and_then(|v| v.as_str()) {
    CellValue::formula(f.to_string())          // discards number/string/boolean/errorValue/dateSerial
}
```

Branch-4 (`valueType` key) *does* read `number`, `string`, `boolean`, `error_value`,
`dateSerial` — but branch-3 returns first, so a formula object never reaches it.

Fix: in branch-3, construct the `Formula` `CellValue` and then fold any supplied scalar
fields onto it (5 lines, reusing existing `CellValue` fields — `CellValue::formula` at
`cell.rs:210` zeroes them all; branch-4 at `cell.rs:403` reads them, but branch-3 returns first).
No new fields, no new types, no signature changes. The getter already consumes them; the writer
arm additionally needs `date_serial` (below).

## Design decisions (minimal-first)

+ **D2a.** Reuse the existing `CellValue` scalar fields (`number`, `string`, `boolean`,
  `error_value`, `date_serial`) as the cache-authoring surface. Do **not** add a dedicated
  `cachedValue` input key. Rationale: zero new Rust fields; writer arm + getter already read
  them; matches the existing `valueType`-shaped input convention.
+ **D1a.** Keep `cachedValue` recalc-only (status quo). Do **not** relax its
  `value_type != "Formula" { return None }` guard (line 459) to also surface disk/Excel-authored
  caches. Rationale: issue #54 is the authoring round-trip + a fixture, **not** a re-definition
  of `cachedValue`. Unifying that getter is a separate, behavior-changing decision (it would
  change the return value of `cachedValue` for every real-Excel formula cell on disk-read); that
  is explicitly deferred, not folded in here.
+ **D3.** Fixture is **ExcelJS-authored** (`{ formula, result }` → ExcelJS emits
  `<f><v>` per `cell-xform.js:158-189`), plus a hand-crafted 200-byte `.xlsx` fixture that
  carries `<f>..</f><v>..</v>` to lock the pure read path independent of the writer/authorship.
  + Debunks issue #54's assumption ("ExcelJS never writes cached `<v>`"); ExcelJS writes `<v>`
    whenever `result` is set. The existing `reader.test.ts` formula tests simply omitted `result`.
+ Fixture assertions target `cell.value` (the cached scalar) **and** `cell.formula`. They
  deliberately do **not** assert `cachedValue`' read-side unification (that would pull D1b into
  scope — issue #55 only fixes the **TS declaration** of the existing getter, not its runtime read
  surface for Excel-authored caches).

## Non-Goals

+ Reconciling excelrs's `cell.value` (returns the bare cached scalar, e.g. `3`) with
  ExcelJS's formula-cell contract (returns the object `{ formula, result }`). ExcelJS parity
  for formula-cell `value` semantics is a separate, broader question; issue #54 preserves the
  existing v2.8.0 disk-read behavior, which returns the cached scalar.
+ Extending `Cell::set_value` beyond branch-3, or adding new `CellValue` variants.
+ Touching the streaming reader / writer (`stream.rs`), the formula-eval feature, or
  `recalculate()`. Out of scope for this slice.

## Changes

+ **Rust** `src/model/cell.rs` `set_value`: in branch-3 (`cell.rs:388`), fold `number`/
  `string`/`boolean`/`error_value`/`date_serial` from the input object onto the `Formula`
  `CellValue`; **and** `src/writer/xlsx.rs:1855` add the missing `date_serial` `<v>` branch to
  the `"Formula"` arm (mirrors the `Date` arm at `xlsx.rs:1916`).
+ **TS** `index.d.ts` + native glue: widen the `Formula` arm of the `CellValue`/`CellValueInput`
  discriminated union to carry optional scalar fields (`number?`, `string?`, `boolean?`,
  `errorValue?`, `dateSerial?`) for `cell.value`; **and** declare the missing `Cell.cachedValue`
  getter (`get cachedValue(): CellValue | null`) — issue #55's TS decl gap (runtime getter already
  exists at `cell.rs:459`). Mirror both in `native.d.ts`. No new exported symbol.
+ **Tests/fixtures**: add ExcelJS-authored cached-formula test (assert `cell.value` +
  `cell.formula` round-trip) and a committed hand-crafted `.xlsx` with `<f>..</f><v>..</v>`
  asserting the pure read path.
+ **Docs**: `docs/spec.md` §10 and `CHANGELOG.md` note the authorable cached-result support.

## Impact

+ **Public API**: additive on the input side (Formula variant gains optional scalar fields).
  No signature, no breaking change.
+ **Disk-read contract**: unchanged (already returns cached scalar for `<v>`).
+ **Writer/read**: no format change; `<f>` and `<v>` already serialized.
+ **Blast radius**: `Cell::set_value` branch-3 (`cell.rs:388`) + the writer `"Formula"` arm
  (`xlsx.rs:1855`, additive `date_serial` branch mirroring the `Date` arm) + the `CellValue` TS
  union + new test/fixture files. `CellValue::formula` signature and the `cached_value` getter
  are **read-only** here — lowest risk slice of issue #54.
