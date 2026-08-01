## Context

`CellType::from_tag(tag: &str) -> Option<CellType>` was added in commit 7d00668
as the single 10-arm lookup table (`src/model/cell.rs:50`). It is used by
`Cell::value_type()` and `Cell::set_value()`.

**Critical detail for the test design**: `CellType` is annotated
`#[napi(string_enum)]` (napi-rs v3). In this version, the macro generates only
`FromNapiValue` / `ToNapiValue` impls for the FFI bridge — it does **not**
generate an `as_str()` or `AsRef<str>` method on the Rust enum itself. The
`val.as_str()` call seen in the generated `FromNapiValue` code operates on a
`String` received from JS, not on a `CellType` value.

Consequence: a pure `CellType → tag → CellType` round-trip via
`variant.as_str()` is **impossible today** without first adding an `as_str()`
method. The issue's step-5 wording ("all 10 tags round-trip") assumed
`as_str()` exists. This design tests `from_tag` directly and via the real
coupling path through `CellValue`, which is where the tag strings actually
live.

## Goals / Non-Goals

**Goals:**

- Test all 10 tag strings map to correct `CellType` variant via `from_tag`
- Test unknown/empty tags return `None`
- Test the real round-trip: `CellType → CellValue constructor → value_type
  string → from_tag → CellType` (the contract that prevents drift)

**Non-Goals:**

- Adding `as_str()` / `AsRef<str>` to `CellType` (would change the enum's
  public surface)
- Migrating `value()` / `csv.rs` / `table.rs` / `xlsx.rs` dispatch tables
  (issue #48 proposal D4 — behavior-dispatch, out of scope)

## Decisions

### D1: Two test functions in the existing `#[cfg(test)] mod tests`

**Direct test** — `test_cell_type_from_tag_all_tags`:
Asserts each of the 10 known tag strings resolves to the expected `CellType`.
Uses an explicit `[(&str, CellType)]` array; if a variant is missing from the
array, the test still passes for what's listed, so this is a *documented*
contract, not exhaustive. The exhaustiveness guard is D2.

**Round-trip test** — `test_cell_type_from_tag_round_trip_via_cell_value`:
For each `CellValue` constructor (number, string, boolean, formula, hyperlink,
rich_text, date), constructs a `CellValue`, extracts `value_type` string, and
asserts `from_tag` parses it back to the matching `CellType`. This tests the
real coupling: if someone adds a `CellValue` constructor that sets a new
`value_type` tag but forgets the `from_tag` arm, this test fails.

**Unknown tag** — both functions assert `from_tag("Unknown") == None` and
`from_tag("") == None`.

### D2: Exhaustiveness is NOT compile-time-guaranteed by the test

`from_tag` itself returns `None` for unknown tags. The `match` inside `from_tag`
has a `_ => return None` arm, so Rust exhaustiveness does NOT force a new arm
when a variant is added. The test array is the manual exhaustiveness guard — a
developer must add the new pair to the test when adding a variant. This is
documented in the test comment.

### D3: Test placement

Existing test module at `src/model/cell.rs:629` (`mod tests`). Same file,
same visibility (`CellType` is `pub`, `from_tag` is `pub(crate)`, both visible
to `#\[cfg(test)]`). No new test file needed.

## Risks / Trade-offs

- **[Test only covers `from_tag`, not `as_str`] → Mitigation**: there is no
  `as_str()` on `CellType`. The round-trip-via-`CellValue` test covers the real
  coupling instead — it's strictly stronger (tests parse + construction
  consistency) than a synthetic `as_str()` round-trip.
- **[Static array not compiler-enforced for exhaustiveness] → Mitigation**:
  test comment documents the need to update the array when adding a variant;
  `from_tag`'s `_` arm returns `None` so missing variants produce a failing
  assertion.
- **[`CellValue` constructors for `Null` and `Merge` are not directly
  constructable**] → Mitigation: `Null` is the default (`CellValue::default()`
  sets `value_type: "Null"`); `Merge` has no constructor. The direct test
  covers `"Null" → CellType::Null` and `"Merge" → CellType::Merge` explicitly
  with literal strings.

## Migration Plan

N/A — test-only change. No production code touched. Deploy = `cargo test`.

## Open Questions

(None — all deferrable concerns resolved above.)
