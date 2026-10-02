# 025 — ARGB / RGB hex colors, no theme color references

**Status:** superseded
**Recorded at:** v0.2.0
**Superseded by:** theme color read v0.6.0, theme color write v0.13.0

## Decision

Colors in the public API are 8-char ARGB or 6-char RGB hex strings. Theme color
references (`theme="N"`) are not supported.

## Context

Theme references require parsing `xl/theme/theme1.xml` and resolving `theme="N"` indexes
through the theme's color scheme. On write, emitting `<color theme="N"/>` requires carrying
the reference rather than the resolved value, or the output stops round-tripping.

## Alternatives considered

- **Expose the raw theme index as a string.** Rejected — a leaked implementation detail
  that callers cannot use without parsing the theme themselves.
- **Resolve to ARGB and never emit references.** That was this decision, and it is what
  caused the write-side problem below.

## Why superseded

Reading theme colors shipped in **v0.6.0** (resolved to ARGB, with the 56-entry indexed
palette and the OOXML tint algorithm). Writing shipped in **v0.13.0**, emitting
`<color theme="N"/>` with `tint`.

**The tension this decision created was real and worth recording:** resolving theme colors
to ARGB on read is right for the public API, but if the writer then emits the resolved
ARGB, a theme-referencing workbook is rewritten with its theming flattened. Preserving the
reference on write required keeping the theme index and tint in the model *in addition to*
the resolved ARGB, with the resolved value retained for the public API. That is why the
model carries both.

## Sources

- `CHANGELOG.md` — v0.6.0 (theme read, indexed palette, tint) and v0.13.0 (theme write).
- `src/model/color.rs` — `ThemeColorScheme`.
- `docs/specs/theme-color-references/`, `docs/specs/indexed-color-references/`.
- OpenSpec archived change `2026-07-13-v0.6.0-theme-color-references`, decision D1.
