## Context

Today `workbook_to_inner_model` builds `inner.worksheets` from calamine `sheet_names()` (display order). 17 per-sheet parsers then read `format!("xl/worksheets/sheet{}.xml", i + 1)` — positional file index. calamine 0.35's `Sheet` struct exposes only `name`/`typ`/`visible` (no file path), so calamine cannot supply the path; we must parse `xl/workbook.xml.rels` ourselves. The writer renumbers sheet files and rIds on output (`src/writer/xlsx.rs:204-205`, `:1078`), so excelrs-written files are always canonical and round-trips self-heal — but third-party reordered inputs are misread. The reader already has rels-parsing machinery (`parse_sheet_rels` for hyperlinks/comments) we can mirror at workbook scope.

## Goals / Non-Goals

**Goals:** one resolver returning display-ordered real paths; thread through all ~20 bulk per-sheet parsers via `&[String]`; have the shared cell-style parser self-resolve (to keep its public signature used by the streaming reader); regression test for reordered input; no public API change.

**Non-Goals:** changing writer canonicalization; altering unrelated parser logic; changing calamine usage; adding dependencies.

## Decisions

1. **Resolver lives in the reader, not calamine.** calamine 0.35 has no path field → hand-parse `xl/workbook.xml.rels`. Reuse the existing `parse_sheet_rels` HashMap-building style.
   - *Alternatives considered*: (a) switch to calamine's lower-level API — rejected, no path exposed; (b) keep positional + document — rejected, leaves silent misattribution.
2. **One resolver → `Vec<String>` of paths in display order, threaded by index.** Build `sheet_paths` once in `workbook_inner_from_bytes`; each bulk parser uses `sheet_paths[i]`. Minimal blast radius (1-line per call site).
   - *Alternative*: pass rId map and resolve inside each parser — rejected, more duplication.
3. **Adopt Full Threading (Option A).** Rejected *compute-per-fn* (each bulk parser already re-opens its own zip; adding a per-fn `workbook.xml`+`workbook.xml.rels` reparse doubles an already-redundant re-open, 20×→40×) and *hybrid* (coupling rId + `state` into one fn merges two distinct concerns). Threading resolves once and is DRY.
4. **Shared cell-style parser self-resolves.** `parse_styles_and_sheet_maps(data, sheet_count: usize)` is `pub` and also called by the streaming reader (`stream.rs`, `stream_handle.rs`) and by tests passing `0`. Keep its signature; inside, call the resolver to fix bulk-read cell-style alignment without breaking the streaming reader or its tests.
5. **Fallback to positional** when rels/rId are absent → preserves behavior on minimal workbooks and avoids panics.
6. **Keep `inner.worksheets` in display order** (calamine). Paths align 1:1 by index.

## Risks / Trade-offs

- [Risk] ~20 call-site edits introduce typos or wrong index → **Mitigation**: mechanical `paths[i]` swap + full round-trip suite + new reordered-input test.
- [Risk] Malformed rels (dangling rId, relative paths) → **Mitigation**: resolve target relative to `xl/`, fall back to positional on any parse failure.
- [Risk] Slight read-time cost (one extra zip entry parse) → **Mitigation**: parse rels once, O(1) lookup per sheet.

## Migration Plan

Additive; no API break. Land as a fast-follow to PR #65 (the D nit is already posted). Rollback = revert the change; old positional behavior returns.

## Open Questions

None that change scope. (A later, separate effort could drive the writer from the same resolver, but that is out of scope here.)
