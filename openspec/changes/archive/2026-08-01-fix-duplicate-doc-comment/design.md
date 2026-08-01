## Context

See proposal.md — a duplicated `///` block sits immediately above `pub fn stream_write_to_file` in `src/stream.rs` (lines 1152-1157 are an exact copy of 1146-1151). `rustc`/`rustdoc` emit no warning for adjacent duplicate doc lines, so it slipped through normal build checks.

## Goals / Non-Goals

**Goals:** single canonical doc-comment for `stream_write_to_file`; zero behavioral change.
**Non-Goals:** restructure the streaming writer; touch `stream_write_to_memory`; fix unrelated doc style.

## Decisions

- **Delete the second block (1152-1157), keep the first (1146-1151).** Rationale: the first occurrence is the canonical doc; the second is the copy. Deleting either yields an identical result, but keeping the earlier location preserves line stability for any in-flight review comments already referencing 1146-1151.
- **Alternative considered:** dedup via `rustfmt`/`cargo fmt`. Rejected: Rust's formatter does not collapse duplicated doc-comment lines, and it is heavier than a one-block delete.

## Risks / Trade-offs

- **Stale line references:** proposal/citations cite current line numbers (1146-1157); the edit shifts later lines by -6. Acceptable — docs-only change with no downstream line-pinned references.
- **Adjacent twin (`stream_write_to_memory`):** an identical duplicate exists and was observed. Risk a reviewer assumes this change handled it. Mitigation: called out in proposal Impact as an explicit out-of-scope follow-up.

## Migration Plan

None — no API or behavior change.

## Open Questions

(none)
