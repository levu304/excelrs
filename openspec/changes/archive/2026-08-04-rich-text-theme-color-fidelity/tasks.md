## 1. Theme plumbing

- [x] 1.1 Load `ThemeColorScheme` once in `workbook_inner_from_bytes` from `xl/theme/theme1.xml` (reuse `ThemeColorScheme::from_xml` + `apply_tint`); fall back to `ThemeColorScheme::default()` when absent.
- [x] 1.2 Thread `&ThemeColorScheme` into `parse_inline_str_rich_text` and `parse_shared_string_rich_text` signatures (both already open the zip — pass scheme in, do not re-load).

## 2. `apply_rpr_child` font resolution

- [x] 2.1 Change signature to `apply_rpr_child(font, elem, has_rpr, scheme: &ThemeColorScheme)`.
- [x] 2.2 `b` / `i` / `u` arms: read `val` attr; set `Some(true)` unless `val ∈ {"0","false","none"}`; `<u val="none">` ⇒ underline `Some(false)`.
- [x] 2.3 `color` arm: read `rgb` (direct) else `theme`+`tint` (scheme.resolve_theme) else `indexed` (scheme.resolve_indexed) else `auto=="1"` ⇒ `"FF000000"`; priority rgb>theme>indexed>auto. Store ARGB on `font.color`.
- [x] 2.4 Update both call sites to pass `scheme`.

## 3. Tests

- [x] 3.1 Rust: `<b val="0"/>`/`<i/>`/`<u val="none"/>` → bold=false, italic=true, underline=false (inline + shared-string fixtures).
- [x] 3.2 Rust: `<color theme="4" tint="0.5"/>` resolves to non-null ARGB; absent theme1.xml falls back to default accent1.
- [x] 3.3 Rust: `<color indexed="8"/>` and `<color auto/>` resolve to non-null ARGB / `"FF000000"`.
- [x] 3.4 JS integration: read ExcelJS/themed shared-strings file, assert `cell.richText[..].font.color` is ARGB string (not null) and off-flags honored.

## 4. Verification

- [x] 4.1 `cargo test --lib` green (existing 416 + new).
- [x] 4.2 `cargo clippy --lib -- -D warnings` clean.
- [x] 4.3 JS test sweep green (existing rich-text suite + 3.4).
