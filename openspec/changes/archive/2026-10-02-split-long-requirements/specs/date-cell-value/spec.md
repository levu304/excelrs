## REMOVED Requirements

### Requirement: Date values are UTC-anchored (ExcelJS parity)

**Reason**: Combined the UTC-anchored serial mapping with its accepted-display-shift consequence and the JS `Date` assignment rule into one over-long requirement.

**Migration**: Replaced by "Date values are UTC-anchored (ExcelJS parity)", "UTC-anchored dates shift the displayed day west of UTC", and "A JS Date assigned to cell.value is read as UTC milliseconds".

## ADDED Requirements

### Requirement: Date cell values use the UTC-anchored Excel serial mapping

`cell.value` for a Date cell and `cell.date` SHALL convert the Excel serial using the
UTC-anchored mapping `ms = (serial - 25569) * 86400000`, and
`serial = ms / 86400000 + 25569`. This matches ExcelJS 4.4 behavior.

#### Scenario: Serial converts to a UTC instant

- **WHEN** a Date cell's Excel serial is read as a `Date`
- **THEN** the result SHALL be the UTC instant given by `ms = (serial - 25569) * 86400000`

#### Scenario: Date converts back to the original serial

- **WHEN** a `Date` is written back to a Date cell
- **THEN** the emitted serial SHALL be `ms / 86400000 + 25569`, round-tripping the input serial

#### Scenario: toISOString returns the correct value

- **WHEN** a Date cell is read and its `Date` is formatted with `toISOString()`
- **THEN** the output SHALL be the UTC-anchored instant

### Requirement: UTC-anchored dates shift the displayed day west of UTC

Because the internal `Date` represents the UTC instant of the serial, local-formatting
methods (`.toString()`, `.toLocaleDateString()`) SHALL shift the displayed day in timezones
west of UTC. This is accepted, documented behavior, not a bug.

#### Scenario: Local formatting shows the shifted day

- **WHEN** a Date cell is read and formatted with `.toLocaleDateString()` in a timezone west of UTC
- **THEN** the displayed day MAY differ from the day implied by the serial in that timezone

### Requirement: A JS Date assigned to cell.value is read as UTC milliseconds

A JS `Date` assigned to `cell.value` SHALL be interpreted by its UTC milliseconds
(`Date.prototype.getTime()`), not by its local calendar fields.

#### Scenario: Local-midnight Date uses its UTC instant

- **WHEN** a JS `Date` whose local calendar day differs from its UTC day is assigned to `cell.value`
- **THEN** the stored value SHALL be derived from the `Date`'s UTC milliseconds