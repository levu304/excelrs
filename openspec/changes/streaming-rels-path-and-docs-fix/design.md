## Context

PR #27 (streaming-hardening) introduced `parse_workbook_sheet_targets`, which
resolves sheet files directly from rels targets via
`format!("xl/{}", target)`. The review (review #4730539157) flagged that an
absolute rels `Target` produces a malformed path. Separately, the cap
constants' doc block has a misattributed line.

## Goals / Non-Goals

**Goals:**

- Make sheet resolution robust to absolute (package-rooted) rels `Target` values.
- Correct the `MAX_EVENTS` documentation.

**Non-Goals:**

- No refactor of the broader streaming path.
- No change to public API or FFI.
- Not addressing the `from_js_value` default change (issue #29) — tracked separately.

## Decisions

1. **Normalize rels target before resolving the sheet path.**
   - Absolute targets (leading `/`) are package-rooted, so `/xl/worksheets/sheet1.xml`
     already denotes the package path `xl/worksheets/sheet1.xml`. Relative targets
     (no leading `/`) are relative to `xl/`, so they need the `xl/` prefix.
   - Use an explicit branch:

     ```rust
     let path = if let Some(pkg) = target.strip_prefix('/') {
         pkg.to_string()            // package-rooted absolute target
     } else {
         format!("xl/{}", target)   // relative to xl/
     };
     result.push((name, path));
     ```

   - **Why this over `target.trim_start_matches('/')` then always prefixing `xl/`:**
     a bare `trim_start_matches('/')` + `format!("xl/{}", …)` would turn
     `/xl/worksheets/sheet1.xml` into `xl/xl/worksheets/sheet1.xml` (double prefix)
     — still wrong. The branch is the minimal correct form.
   - Alternative: `Path::new(target).strip_prefix('/')` — rejected; string ops are
     sufficient and avoid a `Path` dependency for a zip-internal path.

2. **Doc comment move.**
   - Split the doc block: keep the 3 `MAX_ENTRY_BYTES` lines, add the SAX-events line
     as a `///` directly above `const MAX_EVENTS`.
   - **Why:** matches the existing doc style; zero runtime effect.

## Risks / Trade-offs

- [Risk] A rels `Target` that is absolute but rooted outside `xl/` (e.g.
  `/foo/sheet.xml`) is still non-conformant OOXML and would not resolve. →
  Mitigation: the branch is a strict improvement over the current code; full
  normalization of exotic targets is out of scope.
- [Risk] None for the doc move.

## Migration Plan

N/A (internal fix). Ships with the next release; rollback is a plain revert.

## Open Questions

None.
