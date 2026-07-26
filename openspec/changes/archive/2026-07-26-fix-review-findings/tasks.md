## 1. Pattern fill — fix OOXML emission

- [x] 1.1 Write failing test: parse `patternType="gray125"` → `fill.pattern == Some("gray125")`
- [x] 1.2 Write failing test: parse `patternType="lightHorizontal"` → `fill.pattern == Some("lightHorizontal")`
- [x] 1.3 Write failing test: emit with `fill.pattern = Some("gray125")` → output has `patternType="gray125"`
- [x] 1.4 Write failing test: emit with `fill.pattern = None` + has foreground → output has `patternType="solid"`
- [x] 1.5 Write failing test: DXF emit with `fill.pattern` → preserves pattern name
- [x] 1.6 Update `FillKind` — remove `Pattern` variant, update `Display`, `From<&str>`, `Default`, serde
- [x] 1.7 Update reader (`styles.rs`): store raw `patternType` into `fill.pattern`
- [x] 1.8 Update writer (`styles.rs`): use `f.pattern.as_deref().unwrap_or("solid")` for `patternType` in `emit_fills`
- [x] 1.9 Update writer (`styles.rs`): same fix for DXF `emit_dxf` patternFill path
- [x] 1.10 Update writer validation tests for removed `Pattern` variant
- [x] 1.11 Verify `cargo test` passes all tests (new + existing)

## 2. BorderStyleStyle — add missing OOXML variants

- [x] 2.1 Write failing test: parse `style="hair"` → `BorderStyleStyle::Hair`
- [x] 2.2 Write failing test: parse `style="dashDot"` → `BorderStyleStyle::DashDot`
- [x] 2.3 Write failing test: parse `style="dashDotDot"` → `BorderStyleStyle::DashDotDot`
- [x] 2.4 Write failing test: parse `style="mediumDashDot"` → `BorderStyleStyle::MediumDashDot`
- [x] 2.5 Write failing test: parse `style="slantDashDot"` → `BorderStyleStyle::SlantDashDot`
- [x] 2.6 Write failing test: parse `style="mediumDashed"` → `BorderStyleStyle::MediumDashed`
- [x] 2.7 Write failing test: parse `style="mediumDashDotDot"` → `BorderStyleStyle::MediumDashDotDot`
- [x] 2.8 Write failing test: case-insensitive parse `"HAIR"` → `BorderStyleStyle::Hair`
- [x] 2.9 Add 7 new variants to `BorderStyleStyle` enum, `Display`, `From<&str>`, serde
- [x] 2.10 Verify `cargo test` passes all tests

## 3. AlignmentVertical::Middle — fix Display to "center"

- [x] 3.1 Write failing test: `AlignmentVertical::Middle.to_string()` → `"center"`
- [x] 3.2 Write failing test: emit alignment with `vertical: Middle` → xml has `vertical="center"`
- [x] 3.3 Update `AlignmentVertical::Middle` Display from `"middle"` to `"center"`
- [x] 3.4 Remove writer special-case override in `emit_alignment_child` (`writer/styles.rs:670-677`)
- [x] 3.5 Verify `cargo test` passes all tests

## 4. Final verification

- [x] 4.1 `cargo test` — all Rust tests pass (new + existing)
- [x] 4.2 `cargo clippy -- -D warnings` — no warnings
- [x] 4.3 `pnpm test` — all JS integration tests pass
- [x] 4.4 `npx tsc --noEmit` — TypeScript clean
