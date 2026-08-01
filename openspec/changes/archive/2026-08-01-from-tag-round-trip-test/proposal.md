## Why

Commit 7d00668 ("centralize CellType discriminant tag matching via from_tag")
added `CellType::from_tag(tag: &str) -> Option<CellType>` as the single
10-arm lookup table, replacing duplicated string-literal matching in
`value_type()` and `set_value()`. However, no unit test was added for the
new function (issue #48, step 5: "Unit test: all 10 tags round-trip +
unknown → `None`"). Without a test, future `CellType` variant additions can
silently miss the `from_tag` table — exactly the class of drift the original
refactor was meant to prevent. The compiler enforces exhaustiveness on the
`match` inside `from_tag`, but only a round-trip test proves the `as_str`/
`from_tag` pairing is correct and the `None` fallback works for unknown tags.

## What Changes

- Add `#[cfg(test)]` module in `src/model/cell.rs` with a test that:
  - Asserts all 10 `CellType` variants round-trip: `from_tag(v.as_str()) == Some(v)`
  - Asserts an unknown tag string returns `None`
- No production code changes, no API/DTS changes, behavior-neutral.

## Capabilities

### New Capabilities

(None — pure test addition.)

### Modified Capabilities

(None — no spec-level behavior changes. `skip_specs: true`.)

## Impact

- `src/model/cell.rs`: new `#[cfg(test)]` test function only.
- `CellType::as_str()` / `CellType::from_tag()` round-trip contract.
- No JS-facing surface, no `index.d.ts` changes, no runtime behavior change.
- Verification: `cargo test` (Rust unit tests).
