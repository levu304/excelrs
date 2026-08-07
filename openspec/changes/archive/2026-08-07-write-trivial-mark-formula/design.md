## Context

After `cleanup-value-type-from-tag`, every discriminant **read** routes through `CellType::from_tag(&cv.value_type)`. The `value_type` field is still a `String`; `CellValue` exposes constructors (`number`, `string`, `boolean`, `formula`, `hyperlink`, `rich_text`, `date`) but **no method that mutates the discriminant of an existing value**. See proposal.md — Why.

`worksheet.rs::insert_cell_formula` runs after the reader's Pass 1, when `cv` already carries a cached scalar (number/string/etc.). It must add the formula *without* dropping those fields, so `CellValue::formula()` (which builds a fresh value) is unsuitable — hence the raw `cv.value_type = "Formula".to_string()` plus `cv.formula = Some(formula)`.

## Goals / Non-Goals

**Goals:**

- Provide one intent-revealing, type-checked API for marking an existing `CellValue` as a formula.
- Remove the last direct discriminant-string mutation at the call site.

**Non-Goals:**

- Storing `CellType` as an enum on `CellValue` (separate, larger effort — see explore notes).
- Centralizing other discriminant **write** sites (`value_to_cell_value`, reader paths).
- Any behavior, ABI, or serialization change.

## Decisions

1. **Builder shape `mark_formula(mut self, formula: impl Into<String>) -> Self`.**
   Mirrors the existing `validate(self) -> Result<Self, _>` builder on `CellValue` and fits the call site (`cv = cv.mark_formula(formula)` after a `let mut cv`). Alternatives: `&mut self` setter — rejected for ergonomics; free function — rejected, an inherent method keeps the discriminant logic co-located with the other constructors.

2. **Write the `"Formula"` literal inside the builder.**
   This concentrates the one remaining discriminant write into the `CellValue` API instead of the reader. It is the same string already used by `CellValue::formula()` and matched by `from_tag`, so no new vocabulary is introduced. (Full enum storage would later remove the literal entirely — out of scope here.)

## Risks / Trade-offs

- [Tag drift] `mark_formula` hardcodes `"Formula"`, duplicating the `from_tag` table. If `CellType::Formula`'s tag ever changes, this string must change too — same risk every existing constructor carries today. → Mitigation: no change planned; acceptable, documented.
- [Behavior-neutral] Method sets the identical fields (`value_type`, `formula`) as the replaced code, so round-trip, evaluator, and writer behavior are unchanged. → Mitigation: covered by existing test suite; no new tests required beyond confirming green.

## Migration Plan

None — internal refactor, no user-facing or serialized-format change. Rollback = revert the two-file diff.

## Open Questions

None.
