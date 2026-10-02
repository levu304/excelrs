# 008 — Read-only styles in v0.1

**Status:** superseded
**Recorded at:** v0.1.0 (June 2026)
**Superseded by:** style CRUD shipping in v0.2.0 (font, fill, border, alignment, number
  format) and full style round-trip in v0.3.0

## Decision

Ship v0.1 with styles readable but not writable. Defer style CRUD.

## Context

Style CRUD is a large surface — five sub-types, an aggregate `Style`, canonical
serialization rules, and a `cellXfs` index — and getting basic I/O working first was the
right sequencing for a from-scratch port.

## Alternatives considered

- **Styles in v0.1.** Rejected as too much surface before the reader and writer worked.
- **Never.** Not a real option for a drop-in replacement.

## Why superseded

This was a deliberate sequencing decision with a stated expiry, and it expired. Writing
shipped in v0.2.0, reading in v0.3.0. It was correct for the codebase it was made in.

## Sources

- `CHANGELOG.md` — v0.2.0 and v0.3.0 style entries.
- `src/model/style.rs`, `src/writer/styles.rs` — the shipped implementation.
- Rationale recovered from the v1.0.0 specification's one-line table entry.
