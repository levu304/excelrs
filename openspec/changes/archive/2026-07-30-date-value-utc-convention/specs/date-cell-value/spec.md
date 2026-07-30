## ADDED Requirements

### Requirement: Date values are UTC-anchored (ExcelJS parity)

`cell.value` for a Date cell and `cell.date` SHALL convert the Excel serial using a
**UTC-anchored** mapping: `ms = (serial - 25569) * 86400000`, and `serial = ms / 86400000 + 25569`. This matches ExcelJS 4.4 behavior. The internal `Date` therefore represents the **UTC instant** of the serial; `toISOString()` returns the correct value, while local-formatting methods (`.toString()`, `.toLocaleDateString()`) shift the displayed day in timezones west of UTC for date-only serials. This is accepted, documented behavior — not a bug.

A JS `Date` assigned to `cell.value` SHALL be interpreted by its **UTC** milliseconds
(`Date.prototype.getTime()`), not its local calendar fields.

#### Scenario: date-only serial maps to correct UTC instant

- **WHEN** a cell holds serial `45458.0` (2024-06-15, date-only)
- **THEN** `cell.value.toISOString()` SHALL equal `2024-06-15T00:00:00.000Z`

#### Scenario: assigning a UTC-constructed Date round-trips exactly

- **WHEN** `cell.value = new Date(Date.UTC(2026, 0, 15))`
- **THEN** the stored serial SHALL be `46040.0` and reading back yields `toISOString() === '2026-01-15T00:00:00.000Z'`

#### Scenario: local-constructed Date is interpreted as UTC

- **WHEN** `cell.value = new Date(2026, 0, 15)` (local midnight) in a timezone behind UTC
- **THEN** the value SHALL be stored/returned by its UTC milliseconds (matching ExcelJS), so local display may show the prior day; consumers wanting the local calendar date SHALL construct with `Date.UTC(...)`
