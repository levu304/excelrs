## Why

PR #36 added typed Rust enums for string-literal fields but introduced gaps: `FillKind::Pattern` emits invalid OOXML `patternType="pattern"` (not a valid ST_PatternType per §18.18.55), `Fill.pattern` field is dead code never plumbed through reader/writer, missing OOXML border styles silently map to `Thin` on read, and `AlignmentVertical::Middle` Display/emit use inconsistent values ("middle" vs "center"). Fix these issues using TDD approach — write failing tests first, then fix production code.

## What Changes

- **Fix `FillKind::Pattern` OOXML emission** — writer uses `Fill.pattern` (actual OOXML pattern type name) instead of `FillKind` Display for `patternType` attribute. **BREAKING**: removes `FillKind::Pattern` variant (unused, had no valid OOXML mapping)
- **Plumb `Fill.pattern` through reader** — reader stores raw OOXML `patternType` string into `Fill.pattern` field
- **Writer fallback** — when no explicit pattern set but pattern fill is active, fall back to `"solid"` (OOXML default)
- **Add missing OOXML border styles** — add `Hair`, `DashDot`, `DashDotDot`, `MediumDashDot`, `SlantDashDot`, `MediumDashed`, `MediumDashDotDot` to `BorderStyleStyle` enum
- **Fix `AlignmentVertical::Middle`** — Change Display to emit `"center"` (OOXML value) and remove special case in writer
- **TDD approach** — For each fix: write a failing Rust test first, then implement the production code fix

## Capabilities

### New Capabilities

- `pattern-fill`: Correct OOXML pattern fill emission — `Fill.pattern` field plumbed through reader/writer instead of using `FillKind` discriminant for `patternType` attribute
- `border-style-styles`: Complete set of OOXML §18.18.3 border line style variants (12 total)
- `alignment-vertical-fix`: `AlignmentVertical::Middle` Display outputs OOXML-correct `"center"` value

### Modified Capabilities

- *(none — these are all new capabilities under the enum changes from PR #36)*

## Impact

- **src/model/style.rs**: Remove `FillKind::Pattern` variant. Add 7 missing `BorderStyleStyle` variants. Fix `AlignmentVertical::Middle` Display.
- **src/model/fill.rs** (model): Plumb `Fill.pattern` field — already exists, needs reader/writer wiring
- **src/reader/styles.rs**: Store raw OOXML `patternType` into `Fill.pattern` when parsing
- **src/writer/styles.rs**: Use `Fill.pattern` for `patternType` attribute, remove writer special case for `AlignmentVertical::Middle`
- **index.d.ts**: Auto-regenerated. `BorderStyleStyle` gains 7 new variants.
- **Tests**: Write failing test per bug before fixing — `test_parse_pattern_fill_gray125`, `test_emit_pattern_fill_solid`, `test_border_style_hair`, `test_alignment_vertical_center`
