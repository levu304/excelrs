# Contributing & Release Policy

This document records two release decisions made while triaging issue #3. They are
honored on every release so they do not silently regress.

## 1. Linking partial fixes to multi-item issues

Issue #3 tracks several style-system features plus release-hardening items. When a
PR resolves *only some* of the items in a multi-item issue, it must **keep the
issue open** so the remaining work is not lost.

- Use `Refs #N` (or `Refs #N` among other refs) for a partial fix — the issue
  stays open.
- Use `Closes #N` / `Fixes #N` **only** when the issue is *fully* resolved.

Do not use `Closes #N` for a partial fix just because the PR builds; that closes
the issue and hides unfinished items.

## 2. Solid fills use `fill.foreground`, never `fill.color`

The library's `Fill` type has these (relevant) fields: `kind`, `foreground`,
`background`, `pattern`, and gradient fields. There is **no `color` field** on
`Fill`.

To set a solid fill, use:

```js
fill: { kind: "solid", foreground: "FFFF0000" }
```

The correct read-back assertion is `fill.foreground`. Never write or assert
`fill.color` — it does not exist and the API will reject/ignore it. This rule
applies to both code and the release smoke test (which round-trips
`font.bold` + `fill.foreground` through the read path).

## 3. Merged cells: ranges, not per-cell state

Merged ranges are exposed **per-worksheet** via `ws.mergedRanges` (e.g.
`["B2:D4"]`); non-anchor cells inside a merge read back as empty. Unlike
ExcelJS, there is **no per-cell `merge` value type** — do not assert
`cell.value.type === 'merge'`. To test membership, use the query helper:

```js
ws.isMerged(row, col); // 1-indexed; returns the range string or null
```

This keeps a single source of truth (`mergedRanges`) and avoids duplicating
range state on every cell.
