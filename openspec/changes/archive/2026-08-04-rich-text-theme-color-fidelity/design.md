## Context

PR #58 introduced `parse_shared_string_rich_text` + `overlay_shared_string_rich_text` and, for both rich-text parsers, extracted a shared `apply_rpr_child(font, elem, has_rpr)` helper (`src/reader/xlsx.rs:2282`). That helper is the single chokepoint for every run-level `<rPr>` child across inline-string and shared-string paths. The workbook theme-color resolver `ThemeColorScheme::resolve_theme(index, tint)` / `resolve_indexed(index)` already exists on `src/model/color.rs:157` and is already wired into the style-table reader (`src/reader/styles.rs:759` loads `xl/theme/theme1.xml` and resolves cell-font colors). So no new dependency or color math is needed — only plumbing.

See proposal.md — Why.

## Goals / Non-Goals

**Goals**

- One shared helper (`apply_rpr_child`) resolves themed/indexed/auto colors and honors `val` on bold/italic/underline, for both inline and shared-string runs.
- `Font` public shape unchanged (`color: Option<String>` ARGB). `Cell.richText` getter unchanged.
- Theme scheme loaded once per workbook, reused by both parsers — no per-run archive access.

**Non-Goals**

- Writing `<color theme="N"/>` back (writer keeps resolving to ARGB at read time per `theme-color-references` spec; unchanged).
- Distinguishing `<u val="double"/>`/`doubleAccounting` in the bool `underline` field — treated as `Some(true)` (documented ceiling).
- Rich-text color in the **cell-style** (non-rich-text) path — that already resolves via the style-table reader and is untouched.

## Decisions

1. **Load `ThemeColorScheme` once in `workbook_inner_from_bytes`, pass `&ThemeColorScheme` down.** Alternative: re-open archive + re-read theme1.xml inside each parser. Rejected — two parsers × two redundant loads; the single-load approach matches how styles already reuse one scheme. Decision driven by avoiding duplicate theme1.xml parsing.

2. **Plumb scheme to `apply_rpr_child`** by changing its signature to `apply_rpr_child(font, elem, has_rpr, scheme: &ThemeColorScheme)`. Both call sites updated identically — keeps the two parsers byte-for-byte aligned on font resolution (parity is the whole point of Enhancement A).

3. **Resolve `color` priority: `rgb` > `theme` > `indexed` > `auto`** per OOXML. If none present, leave `font.color = None` (existing behavior). `tint` applies only when `theme` present (matches `theme-color-references` semantics); `rgb`/`indexed` ignore `tint` per spec.

4. **`val` semantics**: helper reads `val` attr; `Some(true)` unless `val ∈ {0, false, none}`. `<b/>` with no val ⇒ true (Excel/ExcelJS default). `u="none"` ⇒ underline `Some(false)`.

## Risks / Trade-offs

- [Risk] Theme scheme load failure (no `xl/theme/theme1.xml`) — [Mitigation] `ThemeColorScheme::default()` (OOXML standard 12-slot scheme) is the fallback; `resolve_theme`/`resolve_indexed` never panic, return `Option`. Behavior matches `theme-color-references` scenario C1/C2.
- [Risk] `val` parsing edge cases (e.g. `"true"` vs `"1"` vs `""`). [Mitigation] accept the ECMA-376 boolean set {0,1,false,true} plus `"none"` for underline; anything unrecognized defaults to "on" (stricter than ExcelJS but matches existing rgb-only behavior of "presence ⇒ true").
- [Risk] Inline-string parser currently has NO archive/scheme access. [Mitigation] Decision 1 loads scheme at workbook level and passes it in — inline parser gains scheme for free, no per-cell archive open.
- [Risk] `tint` math divergence. [Mitigation] reuse `apply_tint` inside `ThemeColorScheme::resolve_theme` (already used by style reader); no new tint code.

## Migration Plan

No user-facing API change. No data migration. Readers of previously-`null` rich-text theme colors will now see ARGB strings (improvement, not breakage). Writer unaffected.

## Open Questions

None that gate this change. (The `u val="double"` ceiling in Decision 4 is a known deferral, not an open question.)
