## 1. Core refactor: centralize tag matching

- [x] 1.1 Add `pub(crate) fn from_tag(tag: &str) -> Option<CellType>` to `impl CellType` in `src/model/cell.rs` (10-arm match, `_ => None`)
- [x] 1.2 Replace `value_type()` 11-arm match with `CellType::from_tag(&inner.value.value_type).unwrap_or(CellType::Null)`
- [x] 1.3 Replace `set_value()` 10-arm `matches!(vt, ...)` validation with `CellType::from_tag(vt).is_some()`

## 2. Verify

- [x] 2.1 `cargo clippy -- -D warnings` clean
- [x] 2.2 `cargo test` — all existing Rust unit tests pass (behavior-neutral)
- [x] 2.3 `pnpm test` (vitest) — all JS integration tests pass
