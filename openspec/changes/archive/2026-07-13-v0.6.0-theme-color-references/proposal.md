## Why

excelrs advertises drop-in exceljs compatibility (design principle P1). The
v0.4.0 roadmap (docs/spec.md §9.2.1) listed six headline work units; by v0.5.0
five of them have shipped — row-level style, gradient fills, diagonal borders,
merge cells, and hyperlink/richtext (CHANGELOG.md 0.5.0; git log
`e936e9c feat: v0.5.0 — …`). The single remaining item is **theme color
references**: reading `xl/theme1.xml` and resolving `<color theme="N"/>`.

Today `src/reader/styles.rs` parses `xl/styles.xml` but **silently drops**
theme and indexed colors (the `// theme or indexed colors: skip (color stays
None)` branches at the font, border-side, and fill fg/bg parse sites).
Real Excel files produced by ExcelJS and Excel almost always express brand
colors as `theme="N"` references, so any file with themed styling is read back
with `color = null` — a correctness gap that violates the compatibility
promise for a large class of real-world workbooks. Resolving this closes the
original v0.4.0 roadmap completely.

## What Changes

- Add a `ThemeColorScheme` resolver (new `src/model/color.rs`) that maps
  `theme="N"` → ARGB via `xl/theme1.xml`'s `<a:clrScheme>` (falling back to the
  OOXML default scheme when `theme1.xml` is absent) and `indexed="N"` → ARGB via
  the standard 56-entry system palette (with optional custom `<indexedColors>`
  override).
- Thread the scheme through `parse_style_table` so the three color parse sites
  resolve theme/indexed refs to concrete ARGB strings instead of `None`.
- The model `color` field stays `Option<String>` (ARGB). **No napi/public
  signature changes** — the JS user simply stops getting `null` for themed
  colors and now receives the resolved ARGB hex, exactly as for any other
  color. Writer is unchanged (it already emits `rgb="<ARGB>"`).
- Round-trip fidelity is preserved at the *resolved color* level: a themed
  file read by excelrs yields ARGB, which is written out as ARGB and reads back
  identically. (We do not re-emit `theme="N"` refs; that requires the richer
  exceljs `Color` object and is explicitly out of scope.)

## Capabilities

### New Capabilities

- `theme-color-references`: Reading `xl/theme1.xml` and resolving
  `<color theme="N" [tint="…"]/>` references to ARGB on the style-read path
  (font, fill fg/bg, border sides).
- `indexed-color-references`: Resolving `<color indexed="N"/>` references via
  the standard 56-entry system palette (with optional custom override from
  `theme1.xml`). Bundled because it is the same color-reference family and
  reuses the identical resolver plumbing and parse sites.

### Modified Capabilities

- (implicit) `style-read`: the previously "skipped" theme/indexed color branches
  now resolve. No new FFI signatures.

## Impact

- **Code**: new `src/model/color.rs` (`ThemeColorScheme`); `src/reader/styles.rs`
  (`parse_style_table` gains a `&ThemeColorScheme` param; `parse_color` helper;
  `parse_styles_and_sheet_maps` reads `xl/theme1.xml`); `src/reader/xlsx.rs`
  (passes scheme through). `src/writer/styles.rs` unchanged. `src/model/style.rs`
  doc comment updated only.
- **API**: No JS-facing change. `color` remains a string ARGB.
- **Dependencies**: None added (uses existing `quick_xml`).
- **Tests**: extend `cargo test` (new `color.rs` module + updated
  `reader/styles.rs` tests) and `pnpm test` (new `__test__/theme-color.test.ts`).
- **Spec**: docs/spec.md §6.8 + §9.2.1 updated to mark theme/indexed supported.

## Non-Goals

- No write-side `theme="N"` emission (requires a `Color` object in the public
  API; deferred).
- No tint *lossless* round-trip (tint is applied to produce ARGB; the original
  `tint` factor is not preserved on write).
- No `dxfs`/conditional-formatting theme resolution, no chart colors.

## Alternative scopes considered (rejected for v0.6.0)

1. **Data-validation read/write** — large, separate subsystem (new model type,
   new `xl/worksheets/sheetN.xml` `<dataValidations>` parse + emit, new JS API).
   Belongs in its own release.
2. **Defined names (`definedName`)** — workbook-level `<definedNames>` parsing
   - resolution; orthogonal to the color/theme theme and sizable.
3. **CSV / streaming read-write** — architectural, multi-month effort; not a
   focused unit.
4. **Indexed-color resolution alone (without theme)** — rejected as too narrow;
   theme refs are far more common in real files. Instead indexed is *bundled*
   with theme as the secondary capability (same plumbing, minimal added cost).
