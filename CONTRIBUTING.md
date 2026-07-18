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
