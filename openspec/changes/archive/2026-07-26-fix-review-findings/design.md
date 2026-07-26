## Context

PR #36 (`fix/typed-enum-declarations`) introduced 10 Rust `#[napi(string_enum)]` enums for TypeScript type safety. Three issues were identified in code review:

1. **Pattern fill corruption**: OOXML `patternType` attribute (e.g. `"gray125"`, `"lightGray"`) has zero overlap with `FillKind` discriminant values. The writer uses `f.kind.to_string()` for `patternType`, producing `"pattern"` — not a valid ST_PatternType value (§18.18.55). The `Fill.pattern: Option<String>` field exists on the model but is dead code — never populated by reader, never read by writer.

2. **Missing border styles**: OOXML §18.18.3 defines 12 border line styles. `BorderStyleStyle` only has 6. The remaining 7 (`Hair`, `DashDot`, `DashDotDot`, `MediumDashDot`, `SlantDashDot`, `MediumDashed`, `MediumDashDotDot`) silently map to `Thin` on read via catch-all.

3. **AlignmentVertical Display/emit mismatch**: `AlignmentVertical::Middle` Display writes `"middle"` but OOXML uses `"center"`. Writer has a special-case override at emit time, making Display output dead code for this variant.

TDD approach: write a failing test per issue first, then fix production code.

## Goals / Non-Goals

**Goals:**

- Fix OOXML correctness for pattern fill emission — write valid `patternType` values
- Plumb `Fill.pattern` through reader so round-trip preserves original OOXML pattern name
- Complete `BorderStyleStyle` to cover all 12 OOXML §18.18.3 styles
- Fix `AlignmentVertical::Middle` Display to emit `"center"` and remove writer special case
- Each fix driven by a failing test first

**Non-Goals:**

- No new enums or struct fields beyond the three capabilities
- No changes to napi-rs TS generation (auto-generated)
- No changes to `SheetViewState` empty string (deferred — requires napi-rs workaround)
- No JS/TS consumer API breakage mitigation (PascalCase vs lowercase — separate concern)

## Decisions

### D1: Remove FillKind::Pattern variant

- **Option A (chosen)**: Remove `FillKind::Pattern`. It has no valid OOXML ST_PatternType mapping — all real pattern fills use concrete names from the ST_PatternType enumeration. The old `kind: String` also couldn't represent a valid OOXML pattern fill correctly. Consumer code that sets `fill.kind = FillKind.Pattern` was dead code that emitted invalid OOXML.
- **Option B**: Keep `FillKind::Pattern` but never emit it — treat it as an internal sentinel. This adds dead code with no benefit.
- **Option C**: Keep variant + add `Fill.pattern` plumb through reader/writer. Unnecessary — `kind` is the high-level discriminator (`solid` vs `gradient`), not the OOXML pattern type.

### D2: Plumb Fill.pattern through reader/writer

- **Reader**: When parsing `<patternFill patternType="...">`, store raw OOXML pattern name into both `Fill.kind` (as discriminator) AND `Fill.pattern` (as OOXML name).
- **Writer**: When emitting `patternFill`, use `f.pattern.as_deref().unwrap_or("solid")` for `patternType` attribute — prefer stored OOXML name, fall back to `"solid"`.
- This preserves round-trip fidelity for real OOXML pattern fills like `"gray125"`, `"lightGray"`, etc.

### D3: Add 7 missing BorderStyleStyle variants

- All 7 variants: `Hair`, `DashDot`, `DashDotDot`, `MediumDashDot`, `SlantDashDot`, `MediumDashed`, `MediumDashDotDot`.
- `Display` uses camelCase OOXML names: `"dashDot"`, `"mediumDashDot"`, etc.
- `From<&str>` handles case-insensitive match.
- Catch-all remains `Thin` for unknown values.

### D4: Fix AlignmentVertical::Middle Display

- Change Display from `"middle"` to `"center"` (valid OOXML vertical alignment value).
- Remove the special-case override in `emit_alignment_child` (`writer/styles.rs:670-677`).
- Reader already maps `"center"` → `AlignmentVertical::Middle` via explicit match, so round-trip is correct.

### D5: TDD approach

- Each bug gets a failing Rust test first (`#[test]`).
- Test demonstrates the incorrect behavior before the fix.
- Production code is then changed to make the test pass.
- Verify all existing tests still pass.

## Risks / Trade-offs

- **[Low] Pattern fill corner case**: If a consumer programmatically sets `fill.pattern` to an invalid OOXML value, writer emits it verbatim (garbage in, garbage out). No worse than the old `kind: String` approach.
- **[Low] Border style catch-all**: Unknown OOXML border styles still silently map to `Thin` on read. Acceptable — anyone parsing a future-compatible OOXML file with new styles gets a readable (if slightly different) result.
- **[Medium] Deferred `SheetViewState` empty string**: OOXML allows `state=""` but Rust `string_enum` can't represent empty variant. Module augmentation in `enums.d.ts` handles TS types but Rust reader silently skips empty state. Tag for follow-up.
