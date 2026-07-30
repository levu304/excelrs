# Proposal: Date value UTC convention (document + lock contract)

## Why

PR #46 made `cell.value` return a JS `Date` for Date cells by converting the Excel
serial with `serial_to_millis`/`millis_to_serial` — a **UTC-anchored** pair
(`(serial - 25569) * 86400000` ms). This deliberately matches ExcelJS 4.4, whose
own date conversion uses the same UTC math (and carries the same well-known
off-by-one-day quirk for date-only cells in timezones west of UTC).

The behavior is intentional and round-trips exactly (the serial is preserved), but:

1. It is currently **undocumented** — a consumer formatting a date-only cell with
   local methods (`.toLocaleDateString()`, date-fns, Luxon) sees the wrong day in
   non-UTC zones and has no warning.
2. The reader test only asserts `instanceof Date`, so the **contract can silently
   drift** without a failing test.
3. The existing `date-cell-value` spec has a scenario (`cell.value = new Date(2026, 0, 15)`
   → date-only serial) that the UTC implementation only satisfies under a UTC
   interpretation — the spec never states the convention, so it reads as ambiguous.

This change locks the UTC convention in docs + tests + spec without altering runtime
behavior.

## What Changes

- **Document** the UTC convention in the v2.5.0 migration note (CHANGELOG.md):
  `cell.value` for Date cells is UTC-anchored; format with UTC methods
  (`toISOString()`, `getUTCDate()`) or normalize for local display.
- **Add value-asserting tests** that lock the contract: a known date serial yields
  the expected `toISOString()` UTC instant (not just `instanceof Date`).
- **Reconcile the `date-cell-value` spec** to state the UTC interpretation
  explicitly and make its date-only scenario tz-safe (use a UTC-constructed `Date`).

No runtime/API behavior changes. This is **not** a breaking change.

## Capabilities

- **Modified Capabilities:**
  - `date-cell-value` — clarify that Date values are UTC-anchored (matching ExcelJS);
    the date-only round-trip scenario is defined under UTC interpretation.

## Impact

- **Docs:** CHANGELOG.md v2.5.0 migration note.
- **Tests:** `__test__/reader.test.ts`, `__test__/cell.test.ts` (value assertions).
- **Specs:** delta for `date-cell-value` (clarification only).
- **Code:** none. No dependency, API, or behavioral change.
- **Out of scope:** switching to local-component date construction (Option B) or a
  SheetJS-style `UTC` option — tracked separately if local-correct dates become a
  reported pain point.
