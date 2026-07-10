# v0.6.0 — Theme Color References (TDD task breakdown)

TDD contract: every feature lists its **tests first**, then implementation.
Tests are named and asserted concretely. Implement only to make the listed
tests pass, smallest-first.

## Test budget (target)

- Rust: ~27 new/updated (`color.rs` ~14, `reader/styles.rs` ~10 incl. 1 renamed,
  `reader/xlsx.rs` ~2, `writer/styles.rs` ~1).
- JS: ~9 new in `__test__/theme-color.test.ts`.
- Baseline before start: 146 Rust + 60 JS (CHANGELOG 0.5.0). Target ≈ 173 Rust + 69 JS.

---

## A. `ThemeColorScheme` model (`src/model/color.rs`, new file)

### A-tests (write BEFORE impl)

- [ ] `A1 test_scheme_default_len_12` — `ThemeColorScheme::default().entries.len() == 12`.
- [ ] `A2 test_scheme_default_accent1` — `entries[4] == "4F81BD"`.
- [ ] `A3 test_resolve_theme_lt1` — `resolve_theme(1, None) == Some("FFFFFFFF")` (lt1=FFFFFF, alpha FF).
- [ ] `A4 test_resolve_theme_accent1` — `resolve_theme(4, None) == Some("FF4F81BD")`.
- [ ] `A5 test_resolve_theme_dk2` — `resolve_theme(2, None) == Some("FF1F497D")`.
- [ ] `A6 test_resolve_theme_out_of_range` — `resolve_theme(12, None).is_none()` and `resolve_theme(99,...)==None`.
- [ ] `A7 test_apply_tint_darken` — `apply_tint("FF0000", -0.5) == "800000"` (half).
- [ ] `A8 test_apply_tint_lighten` — `apply_tint("000000", 0.5) == "808080"`.
- [ ] `A9 test_apply_tint_zero_noop` — `apply_tint("4F81BD", 0.0) == "4F81BD"`.
- [ ] `A10 test_resolve_theme_with_tint` — `resolve_theme(4, Some(-0.5)) == Some("FF27425E")` (≈ half of 4F81BD).
- [ ] `A11 test_from_xml_parses_clrScheme` — inline `<a:clrScheme>` with one custom `srgbClr val` parses; that index resolves to it.
- [ ] `A12 test_from_xml_custom_accent1` — custom `accent1` val="FF123456" → `resolve_theme(4,None)==Some("FF123456")`.
- [ ] `A13 test_resolve_indexed_default` — `resolve_indexed(0) == Some(<doc entry 0 ARGB>)`; assert ≥2 known SYSTEM_INDEXED_COLORS entries.
- [ ] `A14 test_resolve_indexed_custom_override` — inline `<a:indexedColors>` with `[2]="FFABCDEF"` → `resolve_indexed(2)==Some("FFABCDEF")`.
- [ ] `A15 test_resolve_indexed_out_of_range` — `resolve_indexed(56).is_none()`.

### A-impl (smallest-first)

- [ ] `A.1` Create `src/model/color.rs` with `ThemeColorScheme { entries:[String;12], indexed:Option<[String;56]> }`, `Default` = ECMA-376 default scheme, `SYSTEM_INDEXED_COLORS:[String;56]` const.
- [ ] `A.2` `resolve_theme(index,u32, tint:Option<f64>) -> Option<String>` (+ `apply_tint`).
- [ ] `A.3` `from_xml(data) -> Result<ThemeColorScheme,ExcelrsError>` parsing `<a:clrScheme>` + optional `<a:indexedColors>` via quick_xml.
- [ ] `A.4` `resolve_indexed(index) -> Option<String>` (custom override else SYSTEM table).
- [ ] `A.5` Wire module into `src/model/mod.rs` (`pub mod color;`).

## B. Reader: resolve colors in `xl/styles.xml` (`src/reader/styles.rs`)

### B-tests

- [ ] `B1 test_parse_font_theme_color` — `theme="4"` font → `fonts[1].color == Some("FF4F81BD")`.
- [ ] `B2 test_parse_border_theme_color` — border `<left><color theme="1"/>` → `borders[1].left.color == Some("FFFFFFFF")`.
- [ ] `B3 test_parse_fill_fg_theme` — `<fgColor theme="6"/>` → `fills[1].foreground == Some("FF4BACC6")` (accent6).
- [ ] `B4 test_parse_fill_bg_theme` — `<bgColor theme="3"/>` → `fills[1].background == Some("FFEEECE1")` (lt2).
- [ ] `B5 test_parse_font_theme_with_tint` — `<color theme="4" tint="-0.5"/>` → resolved ≈ `"FF27425E"`.
- [ ] `B6 test_parse_color_indexed` — `<color indexed="8"/>` → resolved to SYSTEM entry 8 ARGB.
- [ ] `B7 test_parse_no_color_attr_still_none` — `<color rgb="FFFF0000"/>` path unchanged; missing color → None.
- [ ] `B8 test_resolve_theme_not_skipped` — REPLACES existing `test_skip_theme_color`: `theme="1"` now resolves to `Some("FFFFFFFF")` (assert `is_some()`, not `is_none()`).
- [ ] `B9 test_default_scheme_when_theme1_absent` — `parse_style_table(xml, &ThemeColorScheme::default())` with `theme="4"` resolves via default accent1.

### B-impl

- [ ] `B.1` Add `parse_color(attrs, scheme:&ThemeColorScheme) -> Option<String>`: theme→`scheme.resolve_theme`; indexed→`scheme.resolve_indexed`; rgb→uppercased; else None.
- [ ] `B.2` Change `parse_style_table(data, scheme:&ThemeColorScheme)` signature; replace the 3 skip-branches (font color ~L258, border-side color ~L270, fill fg/bg ~L283) with `parse_color`.
- [ ] `B.3` Update the ~13 existing `parse_style_table(...)` call sites in this file's `#[cfg(test)]` to pass `&ThemeColorScheme::default()`.
- [ ] `B.4` Update module doc (remove "Theme colors → skipped" line).

## C. Reader: load `xl/theme1.xml` in the high-level path (`src/reader/styles.rs` + `src/reader/xlsx.rs`)

### C-tests

- [ ] `C1 test_parse_styles_reads_theme1` — `parse_styles_and_sheet_maps` on a zip whose `xl/theme1.xml` has custom accent1 → font `theme="4"` resolves to the custom ARGB (not default).
- [ ] `C2 test_parse_styles_theme1_absent_falls_back` — zip without `theme1.xml` → custom-scheme-free default resolution (no error).

### C-impl

- [ ] `C.1` In `parse_styles_and_sheet_maps`, `read_entry("xl/theme1.xml")`; on Ok parse via `ThemeColorScheme::from_xml`, on Err/missing use `ThemeColorScheme::default()`.
- [ ] `C.2` Pass the resolved scheme into `parse_style_table`.
- [ ] `C.3` `workbook_inner_from_bytes` (xlsx.rs L44) needs no change (scheme flows internally).

## D. Writer passthrough (no behavioral change) (`src/writer/styles.rs`)

### D-tests

- [ ] `D1 test_emit_theme_resolved_argb` — a `Style` whose `font.color` is a theme-resolved ARGB (e.g. "FF4F81BD") emits `<color rgb="FF4F81BD"/>` identically to today (regression guard).

### D-impl

- [ ] `D.1` None required beyond confirming `emit_fonts`/`emit_fills`/`emit_border_side` already pass `color` through. Add the one regression test only.

## E. Integration read (`src/reader/xlsx.rs`)

### E-tests

- [ ] `E1 test_read_themed_xlsx_font_color` — build ExcelJS workbook, set `A1` font `Color{theme:4}`, `writeBuffer()`, load via `read_from_buffer`; assert `getCell('A1').style.font.color == "FF4F81BD"`.
- [ ] `E2 test_read_indexed_xlsx_fill` — ExcelJS cell fill fg `Color{indexed:8}` → excelrs `fill.foreground` == resolved ARGB.

### E-impl

- [ ] `E.1` None beyond C; these verify the end-to-end wiring. If they fail, the bug is in B/C.

## F. JS round-trip tests (`__test__/theme-color.test.ts`, new)

Helper: `async function exceljsThemedToExcelrs(makeWbjs)` — `const buf = await wbjs.xlsx.writeBuffer(); return Workbook.fromBuffer(buf)` (or `new Workbook(); await wb.xlsx.load(buf)`).
Assertions use `cell.style.font.color` / `fill.foreground` / `border.*.color` as **strings** (API stability).

### F-tests

- [ ] `F1` ExcelJS `font.color = { theme: 4 }` → excelrs read → `== "FF4F81BD"`.
- [ ] `F2` ExcelJS `font.color = { theme: 4, tint: -0.5 }` → excelrs read ≈ `"FF27425E"` (tolerance ±1 per channel).
- [ ] `F3` ExcelJS `border.top.color = { theme: 1 }` → excelrs `border.top.color == "FFFFFFFF"`.
- [ ] `F4` ExcelJS `fill.fgColor = { theme: 6 }` → excelrs `fill.foreground == "FF4BACC6"`.
- [ ] `F5` Round-trip: excelrs reads themed → `wb.xlsx.write()` → ExcelJS loads → same ARGB on the cell.
- [ ] `F6` Default scheme: ExcelJS default file (no custom theme) → theme refs resolve to the default palette values.
- [ ] `F7` ExcelJS `fill.fgColor = { indexed: 8 }` → excelrs `fill.foreground` == resolved SYS entry 8.
- [ ] `F8` Custom fixture `fixtures/custom-theme.xlsx` (non-default accents) → excelrs reads the custom ARGB (not the default). Fixture generated by a throwaway script; commit only the `.xlsx`.
- [ ] `F9` API-stability: `typeof cell.style.font.color === "string"` (never an object) — guards against accidental `Color`-object migration.

## G. Docs / spec (follow-up, non-blocking)

- [ ] `G1` Update `src/model/style.rs` header doc: drop "Theme color references are not supported".
- [ ] `G2` `docs/spec.md` §6.8: note theme/indexed colors resolve to ARGB on read.
- [ ] `G3` `docs/spec.md` §9.2.1: mark Theme color references + indexed as shipped in v0.6.0; update §1 version note.
- [ ] `G4` CHANGELOG.md: add 0.6.0 entry (Added: theme + indexed color resolution on read).

## Verification gate (all before merge)

- [ ] `cargo test` green; `cargo clippy -- -D warnings`; `cargo fmt -- --check`.
- [ ] `pnpm test` green (incl. F1–F9).
- [ ] `cargo test` count == baseline 146 + 27 ≈ 173; `pnpm test` == 60 + 9 = 69.
