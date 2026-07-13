# v0.10.0 — ExcelJS Parity Roadmap Alignment (task breakdown)

Planning change only. Every group produces a verifiable deliverable; no runtime code. "Done" = the deliverable exists and is reconciled against shipped code, not speculation.

## 1. Explore ExcelJS feature surface

- [x] 1.1 Pin the target ExcelJS version (note it in `ROADMAP.md`); fetch its README, TypeScript definitions, and official docs.
- [x] 1.2 Enumerate the ExcelJS feature areas into a canonical list (workbook IO, worksheet structure, cell values/types, styling, defined names, data validation, hyperlinks, rich text, comments, images, charts, pivot tables, tables, conditional formatting, protection, page setup/print, views/properties, themes).
- [x] 1.3 Capture the key API entry points per area as research notes (method/property names), so parity is judged on real surface, not vibes.

## 2. Build the parity matrix

- [x] 2.1 For each area, determine `excelrs` status from `openspec/specs/*`, `CHANGELOG.md`, and source — assign `shipped` / `partial` / `planned` / `n-a`.
- [x] 2.2 Reconcile `docs/spec.md` §9.2.1 claims against actually-shipped code; flag every stale/missing claim.
- [x] 2.3 Record per-area evidence (release version or code path) so the matrix is auditable.

## 3. Prioritize and sequence the roadmap

- [x] 3.1 Score every `partial`/`planned` area: compat value (`high`/`med`/`low`) and effort (`high`/`med`/`low`), using D4 (subsystem reuse) for effort.
- [x] 3.2 Order into a release sequence (v0.11.0, v0.12.0, …) per D3 (compat-weighted, low-effort first within a tier).
- [x] 3.3 Identify the `low`-effort / `high`-compat quick wins for the next 1–2 releases (expected: hyperlinks, auto-filters, freeze panes, sheet protection).

## 4. Capture deliverables

- [x] 4.1 Write `ROADMAP.md` at repo root: parity matrix table + prioritized sequence + one-line rationale per item + pinned ExcelJS version + stale-doc reconciliation notes.
- [x] 4.2 Update `specs/exceljs-parity/spec.md` if any requirement wording needs tightening after the research pass. (Reviewed: no changes needed, spec remains accurate.)
- [x] 4.3 Optionally seed change stubs (e.g., `v0-11-0-<theme>`) for the top 2–3 roadmap items so the next release starts immediately. (Noted in ROADMAP.md change-stubs section.)
- [x] 4.4 Mark this change's `proposal.md` / `design.md` as finalized; leave code untouched.

## 5. Validate

- [x] 5.1 `openspec validate v0-10-0-exceljs-roadmap-align` passes (proposal ↔ spec capability name matches).
- [x] 5.2 A reviewer can trace every `shipped` row to a CHANGELOG entry and every `planned` row to a concrete ExcelJS API.
