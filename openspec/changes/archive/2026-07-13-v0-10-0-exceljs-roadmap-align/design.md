## Context

`excelrs` is a Rust + NAPI Excel library that mirrors the ExcelJS API surface (its P1 design principle). Releases to date were each scoped to one themed feature:

- v0.2–v0.6: styles (read + write), merge cells, gradient fills, diagonal borders, theme-color read
- v0.7.0: defined names (named ranges)
- v0.8.0: data validation (read/write)
- v0.9.0: CSV read/write via `wb.csv`

The original roadmap that justified this sequencing (`docs/spec.md` §9.2.1) was last reconciled at v0.6.0 and is now stale: it does not account for v0.7–v0.9, and it predates known gaps (images, comments, hyperlinks, rich text, charts, pivot tables, conditional formatting, protection, page setup, freeze panes, auto-filter). There is no current, evidence-based parity map and no prioritized plan for what to port next. This change fixes that as pure planning work — it ships no code.

## Goals / Non-Goals

**Goals:**

- Produce an accurate ExcelJS → excelrs parity matrix, derived from actual shipped code (not the stale doc).
- Produce a prioritized, release-sequenced porting roadmap (v0.11.0+) that deliberately closes the compatibility gap.
- Establish `exceljs-parity` as the durable, versioned contract future releases MODIFY.

**Non-Goals:**

- Implementing any ported feature — deferred to follow-on releases (v0.11.0+).
- Writing or changing runtime code, FFI signatures, or public API.
- Building a formula-evaluation engine, chart-rendering engine, or pivot engine — those are roadmap *candidates*, not this change's output.

## Decisions

### D1. Source of truth is current ExcelJS, cross-checked against shipped excelrs

The parity matrix is built from ExcelJS's current published API (pinned npm version: README + TypeScript definitions + official docs) and verified against `excelrs`'s real implementation in `openspec/specs/*`, `CHANGELOG.md`, and source — **not** against the stale `docs/spec.md` §9.2.1 roadmap. The doc is treated as a claim to be validated, not a source.

### D2. Fixed status vocabulary

Each area is one of: `shipped` (fully usable), `partial` (some but not all of the area works), `planned` (not implemented, targeted), `n-a` (explicitly out of scope for the drop-in-compat promise). Avoids vague "supported-ish" labels.

### D3. Compat-weighted prioritization

Roadmap order = priority where **compat value dominates effort**. Formula (coarse): sort by `compat_value` (high → low); within a tier, prefer `low` effort before `high`. Rationale: the product promise is drop-in ExcelJS compatibility, so closing visible gaps (e.g., hyperlinks, comments, images — all reusing existing OOXML read/write plumbing) beats speculative large subsystems (charts, pivot) even when the latter is "more impressive."

### D4. Effort estimated by subsystem reuse

- `low`/`med`: features that ride existing OOXML parts and read/write plumbing (hyperlinks, comments, images, sheet/auto-filter, freeze panes, page setup, protection).
- `high`: features needing new engines or large models (charts, pivot tables, conditional formatting, formula evaluation, streaming IO).

### D5. Deliverable shape

Primary output is `ROADMAP.md` at repo root (parity matrix + prioritized sequence + one-line rationale per item). The `exceljs-parity` spec is the machine-checkable contract; `ROADMAP.md` is the human-readable view. Optionally seed change stubs for the top 2–3 roadmap items so v0.11.0 can start immediately.

## Risks / Trade-offs

- **ExcelJS surface is large and evolves** → pin a specific ExcelJS version for the comparison; re-run per major release.
- **Effort estimates are coarse** → re-score at each release; a "low" item that turns hard slides but does not block the sequence.
- **Scope creep (implementing during research)** → explicitly deferred; this change stops at the matrix + roadmap.
- **Over-indexing on parity breadth** → compat value weighting keeps effort-heavy, rarely-used subsystems (charts/pivot) lower in the sequence even if ExcelJS exposes them.

## Open Questions

- Which `low`-effort, `high`-compat quick wins should anchor v0.11.0? (resolved during the research pass — likely hyperlinks + comments + images, all plumbing-reuse.)
- Is streaming reader/writer (`exceljs` stream API) in or out for v1? (Tentatively `n-a` / deferred — perf-oriented, not core drop-in compat.)
- Should `partial` styling items (gradient fills / diagonal borders are read-only in `excelrs`) be promoted to `shipped` with caveats or kept `partial`? (Resolved by the matrix pass against actual read/write coverage.)
