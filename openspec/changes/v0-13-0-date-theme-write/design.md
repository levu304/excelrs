## Context

Two confirmed facts from the codebase (roadmap exploration, recommendations #2/#4):

- **Theme colors are flattened on write.** `reader/styles.rs::parse_color` resolves
  `<color theme="N"/>` (+`tint`) → resolved ARGB and stores it in a plain
  `Option<String>` ARGB field. Neither `writer/styles.rs` nor `writer/xlsx.rs`
  emits a `theme` attribute — only the flattened ARGB. So a themed file read then
  written loses the theme link. The existing `theme-color-references` spec
  (shipped v0.6.0) is **read-only**.
- **`Date` is not a value type.** `model/cell.rs::CellValue` is a struct keyed by a
  `value_type` discriminant (`"Number" | "String" | "Boolean" | "Formula" |
  "Error" | "Null"`) with one `Option` field per variant. There is no `Date`
  variant and no `chrono` usage in the value path. `chrono = "0.4"` is already a
  dependency (via calamine), so the type is available.

The change seeds v0.13.0 with these two as **separate tracks** because they have
disjoint risk: Track A is an OOXML *element* change on existing parts; Track B is
a *core napi type-bridge* change to the value model.

## Goals / Non-Goals

**Goals:**

- Track A: writer emits `<color theme="N"/>` (+ optional `tint`) when a color
  originated from a theme reference, preserving the link on round-trip.
- Track B: a `Date` cell value survives read→write as a JS `Date`, using the
  Excel serial-number + numFmt convention.
- Keep each track independently shippable/versionable.

**Non-Goals:**

- No new **public** color API (no `theme`/`tint` getters exposed to JS) — the
  existing "color is a plain ARGB string" contract is preserved.
- No formula evaluation, no streaming, no charts/pivots/tables (out of scope per
  roadmap `n-a` bucket).
- No theme **editing** UI — we only preserve theme links we read.
- No global "compat flag" to revert Date read behavior (see D4).

## Decisions

### D1 — Theme-color write via an internal `Color` struct (no public API change)

Introduce `pub struct Color { rgb: Option<String>, theme: Option<u8>, tint:
Option<f64> }` in `model/color.rs`. Replace the plain `Option<String>` color
fields in `Font`/`Fill`/`Border` models with `Option<Color>`.

- **Reader (`parse_color`):** continue to resolve theme→ARGB into `rgb` (so the
  public ARGB string is unchanged), **and** retain `theme` + `tint` for
  write-back.
- **Writer:** if `theme.is_some()`, emit `<color theme="N"/>` (+ `tint` when
  present); otherwise emit `<color rgb="..."/>` as today.
- **napi:** `Color` serializes to the resolved ARGB **string** (same as today),
  so `cell.style.font.color` stays a string — honors the existing spec's "no
  public API change for colors" requirement while fixing the round-trip.

*Alternative considered:* expose `theme`/`tint` to JS (richer API, lets users
author theme colors). Rejected — more surface, more spec churn, and not needed
for round-trip parity; can be a later capability if requested.

### D2 — Date as a `CellValue` field bridged to `JS Date`

Add `pub date: Option<chrono::NaiveDateTime>` to `CellValue` with
`value_type: "Date"`. Map to/from `napi::JsDate` in the FFI layer.

- **Read:** when a numeric cell carries a date-like `numFmt`, convert the Excel
  serial (days since 1899-12-30, fractional part = time) → `NaiveDateTime` and
  store as `Date`. Cells without a date numFmt stay `Number`.
- **Write:** a `Date` value → Excel serial number; if the cell/column has no
  numFmt, assign a default date format (`yyyy-mm-dd` for date-only,
  `yyyy-mm-dd hh:mm:ss` when the time component is non-zero).
- **numFmt date detection** is necessarily a heuristic (`is_date_format(fmt)`
  checks for `y/m/d/h/s` tokens).
  - `ponytail`: heuristic covers the common built-in + custom formats; known
    ceiling = locale/custom formats using non-Latin tokens won't be detected →
    such cells fall back to `Number`. Upgrade path: a format-class table from
    `styles.xml` `<numFmt>` ids, if a real gap is reported.

*Alternative considered:* store dates internally as the raw serial number +
`is_date: bool` instead of `NaiveDateTime`. Rejected — `NaiveDateTime` keeps the
Rust type honest and avoids re-implementing serial math at every boundary; chrono
already provides `serial ↔ NaiveDateTime`.

### D3 — Two tracks, one change, independent blast radii

Proposal carries both tracks in a single seed change, but tasks are split and
design is split. Either track can be held/versioned separately (e.g. Date as its
own minor) without reopening the other.

### D4 — Date read behavior change is accepted, not gated

Today a date cell reads back as an ISO-8601 string or number; after D2 it reads
back as a `JS Date`. This is a **behavior change for existing consumers**. We
accept it as the intended fix (the whole point is Date preservation) and document
it with a semver-minor note rather than adding a global compat flag.

- *Alternative considered:* opt-in flag (`{ preserveDates: false }`). Rejected —
  adds API surface + branching for a behavior most ExcelJS users *want*; YAGNI
  until someone asks.

## Risks / Trade-offs

- **[Risk] Date heuristic misclassifies a numeric cell as a date** (e.g. a serial
  ID with a custom numFmt) → Mitigation: only treat as date when numFmt contains
  explicit date/time tokens; `Number` is the safe default.
- **[Risk] Theme write emits `theme="N"` but the target workbook lacks/owns a
  different `theme1.xml`** → on re-read the value re-resolves against whatever
  theme is present (same as Excel), acceptable.
- **[Risk] Changing `Option<String>` color fields to `Option<Color>` ripples
  through `Font`/`Fill`/`Border` reader+writer+napi mappings** → Mitigation: the
  napi mapping collapses `Color` → ARGB string, so JS-side callers are unaffected;
  scope is contained to the color type and its three consumers.
- **[Trade-off] Not exposing theme to JS** means users still can't *author* theme
  colors — accepted for v0.13.0; tracked as a later capability.

## Migration Plan

1. Land Track A (theme-color write) — internal-only, no public contract change,
   low risk; can ship first.
2. Land Track B (Date) — bump minor; update `README.md` limitations + `ROADMAP.md`
   v0.13.0 row; add round-trip fixtures.
3. **Rollback:** both changes are additive/round-trip-preserving; revert the
   relevant commit. Track B's read-behavior change reverts cleanly to string/number
   output.
4. **Validation:** round-trip fixtures — (a) themed file: read→write→read yields
   identical `theme="N"` attributes; (b) date-heavy sheet: a `Date` written then
   read back equals the original `Date`.

## Open Questions

- Should the default Date write numFmt match ExcelJS's default
  (`mm-dd-yy` / `m/d/yy h:mm`) for closer drop-in parity, or ISO (`yyyy-mm-dd`)?
  (Recommend matching ExcelJS default for parity; confirm during implementation.)
- Do we want `Date` values to also populate `cell.value` as `Date` while keeping
  `cell.text` as the formatted string? (Likely yes — mirrors ExcelJS.)
