## Why

excelrs v0.11–v0.12 added **reads** of richer content (theme colors, gradients,
diagonals, rich text, hyperlinks), but each read-only feature silently opens a
round-trip hole for a *drop-in* library: read a styled file, write it back, lose
data. Two concrete gaps remain for v0.13.0:

1. **Theme colors are flattened on write.** The reader resolves `<color
   theme="N"/>` → ARGB (confirmed in `reader/styles.rs`), and the writer emits
   only the resolved ARGB — it never emits `<color theme="N"/>` (confirmed:
   neither `writer/styles.rs` nor `writer/xlsx.rs` contains a theme-attribute
   emit). The semantic theme link is destroyed on round-trip, so later theme
   edits no longer propagate.
2. **JS `Date` is not a model value type at all.** `src/model/cell.rs` has zero
   `Date`/`chrono` variants (verified). Excel dates are serial-number + numFmt,
   and JS `Date` is nowhere in the value bridge, so date cells round-trip as
   plain numbers/strings — breaking the single most common real-world ExcelJS
   workload.

These two have very different risk shapes: Track A is an OOXML *element
addition* on an existing part; Track B is a *core napi type-bridge* touching the
value model. Scoping them as two explicit tracks inside one v0.13.0 seed change
keeps each blast radius isolated while seeding the version's direction
(recommendations #2 and #4 from the roadmap exploration).

## What Changes

- **Track A — OOXML element (theme-color write):** the writer SHALL emit
  `<color theme="N"/>` (with optional `tint`) for theme-resolved colors instead
  of flattening to ARGB, preserving the theme link on write. Reader unchanged.
- **Track B — core value-type bridge (JS Date preservation):** introduce a
  `Date` cell value variant bridged across the napi boundary (`Rust chrono`
  `NaiveDateTime`/`DateTime` ↔ `JS Date`), so dates survive a read→write
  round-trip as dates.
  - This changes how **existing** date cells are presented on read (today a
    date serializes to an ISO-8601 string or number; after this change it
    round-trips as a `Date`). Flagged as a potential **BREAKING** behavior change
    for consumers relying on the current string/number form — warrants a semver
    note and explicit design coverage.
- Both tracks live in one change but are kept structurally separate (own tasks,
  own design subsections) so either can ship or version independently.

## Capabilities

### New Capabilities

- `date-cell-value`: First-class JS `Date` cell value type, bridged across the
  napi boundary and preserved on read→write round-trip (serial number + numFmt
  convention preserved).

### Modified Capabilities

- `theme-color-references`: extends the existing read-only spec (shipped v0.6.0)
  to add a **WRITE** requirement — emit `<color theme="N"/>` (+ optional `tint`)
  rather than flattening to resolved ARGB, so the theme link survives a
  round-trip.

## Impact

- **Model:** `src/model/cell.rs` (new `Date` variant), `src/model/color.rs`
  (carry theme index + tint instead of only ARGB).
- **Writer:** `src/writer/styles.rs`, `src/writer/xlsx.rs` (emit `theme`
  attribute on color emission).
- **Reader:** `src/reader/xlsx.rs` (date detection / `Date` construction on
  read), `src/reader/styles.rs` (already resolves theme → ARGB; needs to also
  retain the theme index for write-back).
- **FFI / public API:** `src/lib.rs` napi bindings + `index.d.ts` / `native.d.ts`
  (`CellValue` gains a `Date` shape; color may expose theme index).
- **Tests / fixtures:** `fixtures/`, `__test__/` (round-trip fixtures for themed
  files and date-heavy sheets).
- **Docs:** `ROADMAP.md` (v0.13.0 row) and `README.md` limitations.
