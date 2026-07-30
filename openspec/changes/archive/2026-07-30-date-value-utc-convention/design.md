## Context

PR #46 made `cell.value` return a JS `Date` for Date cells. The conversion
(`src/model/cell.rs`) uses the UTC-anchored pair `serial_to_millis` /
`millis_to_serial`, producing `ms = (serial - 25569) * 86400000`. This is the same
math ExcelJS 4.4 uses, and the PR body explicitly states "Matches ExcelJS 4.4
behavior." The serial round-trips exactly; the only visible effect is that a
date-only cell displays one day early in zones west of UTC when formatted with local
methods — an inherent property of UTC-anchored dates, not a defect.

The behavior is already shipped. This change adds **no runtime logic**; it captures
the convention in docs, tests, and the `date-cell-value` spec so the contract is
explicit and cannot drift.

## Goals / Non-Goals

**Goals:**

- State the UTC convention clearly in the v2.5.0 migration note.
- Lock the contract with a value-asserting test (`toISOString()` of a known serial),
  replacing the current `instanceof Date`-only assertion.
- Reconcile the `date-cell-value` spec so its date-only scenario is unambiguous
  (UTC interpretation).

**Non-Goals:**

- Changing `cell.value` to local-component construction (Option B).
- Adding a SheetJS-style `UTC` toggle option.
- Any modification to `serial_to_millis` / `millis_to_serial` or `napi` date calls.

## Decisions

- **Keep UTC (Option A) over local components (Option B).** Option B (decompose
  serial → local `Y/M/D/H/M/S`, build `new Date(y,m,d,...)`, decompose on write)
  would fix the local display but: (1) diverges from the PR's stated ExcelJS-4.4
  goal; (2) needs `run_script`-per-cell (perf) or a `libc`/tz dependency; (3) must
  replicate Excel's 1900 fake-leap-day; (4) `napi` `create_date` takes ms only, so
  local anchoring requires deriving a tz offset. The off-by-one is a *known,
  accepted* ExcelJS quirk, so ExcelJS-migrants already expect it. Proportionate
  choice is to document, not re-engineer.
- **Value-asserting test** pins the UTC instant (`toISOString()`) rather than the
  type, so a future drift in the conversion math fails CI.
- **Spec clarification** records the convention as a requirement so it survives
  archiving into `openspec/specs/date-cell-value`.

## Risks / Trade-offs

- [Risk] Date-only cells still display a shifted day in non-UTC local formatting.
  → Mitigation: documented in CHANGELOG; consumers use `toISOString()` / `getUTC*`
  or normalize. Matches ExcelJS, so expected by migrants.
- [Risk] `cell.value = new Date(2026, 0, 15)` (natural local constructor) is read as
  a UTC instant, so in non-UTC zones it stores a serial with a fractional time
  component. → Mitigation: doc note recommends `Date.UTC(...)` for exact calendar
  dates; consistent with ExcelJS.
- [Risk] Test relies on a hardcoded serial→UTC mapping that must stay in sync with
  `serial_to_millis`. → Mitigation: one shared constant/comment; trivial to update.

## Migration Plan

No code/migration. Docs + tests + spec only. Rollback = revert the three files.

## Open Questions

- If local-correct date display becomes a frequently reported pain, revisit Option B
  or a SheetJS-style `UTC` option (separate change).
