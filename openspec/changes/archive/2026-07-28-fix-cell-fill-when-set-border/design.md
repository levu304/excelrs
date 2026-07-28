## Context

**Current state:** `emit_fills` in `src/writer/styles.rs` writes the default fill (index 0, `Fill::default()`) as:

```xml
<fill><patternFill patternType="solid"/></fill>
```

When a user sets `cell.style = { border: { bottom: { style: "thick" } } }` without a `fill`, the style's `fill` field is `None`. `build_style_table` maps `None` to fill index 0 — the default fill. Since the default fill emits `patternType="solid"`, Excel treats the cell as having an explicit solid fill (even without a foreground color), rendering it visually filled.

**Impact:** Every border-only style in every workbook produces corrupted output. This is a high-severity UI bug.

**Fix scope:** One-character change in emit logic. TDD approach: write test first, then fix, then verify.

## Goals / Non-Goals

**Goals:**

- Default/no-fill fill entry (index 0) emits `<patternFill patternType="none"/>` instead of `<patternFill patternType="solid"/>`
- Existing tests continue to pass
- Round-trip safety: read→write preserves fill semantics

**Non-Goals:**

- No API surface changes (structs, napi bindings, types)
- No reader changes
- No style validation changes

## Decisions

**Decision 1: Fix in `emit_fills` emit logic (not `Fill::default`)**

The bug is that the writer's fallback default for `pattern` is `"solid"` when no foreground/background is set. The `Fill::default()` struct has `kind: FillKind::None`, `pattern: None` — that's semantically correct. The fix belongs in the emission logic.

- Option A: Change `Fill::default()` to `pattern: Some("none")` — would work but mixes concerns (model vs emission).
- Option B: Change the fallback in `emit_fills` from `"solid"` to `"none"` on line 538 — targeted fix at the exact emission point. **Chosen.**

Rationale: The `else` branch (no fg, no bg) only handles empty/no-fill fills. `patternType="none"` is the only semantically correct emission for a fill with no colors.

**Decision 2: TDD — add failing test before fix**

Write a test that asserts the default fill XML contains `patternType="none"`, run it (fails with current `"solid"`), apply the fix, verify test passes. Then add a writer round-trip test that writes a workbook with border-only cells and reads it back asserting fills are absent.

## Risks / Trade-offs

- **Minimal risk** — one-character change in a well-understood code path.
- **Round-trip concern:** Reader already parses `patternType="none"` correctly and stores `pattern: Some("none")`. Writer reads `pattern` back. So any fill that was correctly read from an existing file round-trips fine. The bug only affects fills created from scratch (where `pattern` is `None`).
- **Edge case:** If someone explicitly sets `fill: { kind: "none", pattern: "cross" }` with no foreground, they'd get `patternType="cross"` — that's their explicit choice. The fallback `"none"` only kicks in when `pattern` is `None`.
