# Architecture Decision Records — index

Every design decision behind excelrs, with its current standing. One file per decision,
numbered in the order the decisions were made.

**This record answers *why the system is shaped this way*.** It deliberately does not
describe what the system does — that is the job of `openspec/specs/`, which is CI-gated by
`openspec validate --strict`. No requirement in this repository restates a list, count, or
set that already exists in a machine-readable source; see the `platform-targets` capability
for the pattern.

Statuses are exactly two: **current** (governs today) and **superseded** (replaced — the
file stays, naming what replaced it). A superseded decision is never deleted: knowing what
was decided, and why it stopped holding, is usually the fastest way to understand the code
that replaced it.

## Index

| # | Decision | Status | Note |
| --- | --- | --- | --- |
| [001](001-target-exceljs-api-compatibility.md) | Target the ExcelJS API surface | current | |
| [002](002-monolithic-crate.md) | Monolithic crate | superseded | Its own 5K LoC trigger fired un-actioned; the crate-split lever no longer fit. Replaced by a file-size trigger. |
| [003](003-calamine-read-handrolled-write.md) | calamine for read, hand-roll for write | current | Scope corrected: covers the in-memory path only; streaming is hand-rolled SAX. |
| [004](004-tokio-based-async.md) | Tokio-based async | current | |
| [005](005-dual-mit-apache-2-license.md) | Dual MIT / Apache-2.0 license | current | |
| [006](006-node20-napi9.md) | Node.js 20+ (NAPI 9) | current | Corrected: no `engines` field is published, and `napi5` is a floor, not a NAPI 9 baseline. |
| [007](007-manual-tag-release.md) | Manual tag-driven release | current | |
| [008](008-read-only-styles-v01.md) | Read-only styles in v0.1 | superseded | Style CRUD shipped v0.2.0. |
| [009](009-formula-preservation-not-evaluation.md) | Formula preservation, not evaluation | superseded | The "separate product" was partly built: `recalculate()` shipped v2.9.0. |
| [010](010-tier-1-platforms-only.md) | Tier 1 platforms only | superseded | The target list is now the `release.yml` matrix, not a specification. |
| [011](011-flat-napi-object-cellvalue.md) | Flat `#[napi(object)]` struct for `CellValue` | current | P0 blocker from the pre-implementation review. |
| [012](012-thin-js-glue.md) | Hand-maintained JS glue for method overloading | current | P0 blocker. Size corrected: 621 lines, not ~40. |
| [013](013-serde-json-value-cell-setter.md) | `serde_json::Value` for the cell value setter | current | |
| [014](014-napi-v3.md) | napi-rs v3 and `@napi-rs/cli` v3 | current | |
| [015](015-calamine-worksheet-formula.md) | Read formulas via `worksheet_formula()` | current | In-memory path; streaming resolves shared formulas itself. |
| [016](016-dimension-element.md) | Emit `<dimension ref="..."/>` in sheet XML | current | |
| [017](017-remove-dead-cellvalue-variants-v01.md) | Remove `Merge` / `RichText` / `Hyperlink` / `SharedString` from v0.1 | superseded | All four shipped by v0.12.0. |
| [018](018-date-detection-via-calamine.md) | Date detection via calamine's format IDs | current | Corrected: detection is entirely calamine's; excelrs holds no format-ID logic. |
| [019](019-clone-on-read-mutation-semantics.md) | Clone-on-read mutation semantics | superseded | Resolved v0.4.0 via `Arc<Mutex<CellInner>>`. |
| [020](020-napi-constructor-attribute.md) | `#[napi(constructor)]` on `new()` methods | current | |
| [021](021-napi-setter-attribute.md) | `#[napi(setter)]`, not `#[napi(set)]` | current | |
| [022](022-napi-build-version.md) | `napi-build` in build-dependencies | current | Corrected: `Cargo.toml` carries `"2"`, not `"3"`. |
| [023](023-release-binary-size.md) | Release binary size | superseded | ~440 KB figure is ~40x stale; the number is dropped, LTO and strip remain. |
| [024](024-style-write-only-v02.md) | Style write-only in v0.2.0 | superseded | Style reading shipped v0.3.0. |
| [025](025-argb-rgb-no-theme-colors.md) | ARGB / RGB hex colors, no theme references | superseded | Theme read v0.6.0, theme write v0.13.0. |
| [026](026-style-setter-serde-json.md) | Style setter via `serde_json::Value` | current | |
| [027](027-cellxfs-btreemap-dedup.md) | `BTreeMap` dedup for `cellXfs` | current | Cited in-code at `src/writer/styles.rs:1,71`. |
| [028](028-streaming-write-buffering.md) | Streaming write buffering strategy | current | Renumbered from a colliding `005`. |

## Conventions

- **Numbering** is the order decisions were made. The 1–27 space comes from the v1.0.0
  specification's ADR table; those numbers are cited in `CHANGELOG.md` and in code comments
  and are kept resolvable.
- **Provenance.** Every file ends with a `Sources` section. Files marked *recovered* were
  reconstructed after the fact, not authored at decision time — the `reconstruct-design-record`
  change's design decision D3 requires that distinction to stay visible. Where a decision's
  full rationale could not be recovered, the file says so rather than inferring one.
- **Corrections** (a decision that still governs but whose stated figures were wrong) are
  marked in the file and in the index. A reader comparing a decision against the code should
  be able to see that they once disagreed.

## Related records

| Record | Holds |
| --- | --- |
| `openspec/specs/*` | What the system does. The behavior contract; CI-gated. |
| `docs/spec.md` | Design rationale that no requirement can state: FFI constraints, crate and module architecture, rejected alternatives. |
| `CHANGELOG.md` | What shipped, and in which version. |
| `ROADMAP.md` | Feature-area status and prioritization. Hand-maintained. |
| `docs/architecture-review.md`, `docs/architecture-review-pass2.md` | Pre-implementation review of spec v1.0.0/v1.1.0. Primary source for the P0/P1 findings cited throughout this record. |

## Related archived decision corpus

The 77 archived change directories under `openspec/changes/archive/` each carry a
`design.md` with numbered decisions (`### D1.`, `### D2.`, ...). That is **~118 further
decisions**, addressed by their change directory and date rather than lifted into this
index. They are not transcribed here on purpose: the archive path already locates them, and
mirroring 77 directories into a flat index would create a second place to maintain for no
gain in reachability.
