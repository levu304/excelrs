# Spec Delta

## ADDED Requirements

### Requirement: Column descriptors below the valid range are ignored

The `<cols>` reader SHALL ignore column descriptors whose range falls below column 1. No column definition with `col_num < 1` SHALL be created from file input, and such descriptors SHALL NOT reappear in written output on round-trip.

#### Scenario: Zero-based width descriptor is ignored

- **WHEN** worksheet XML contains `<col min="0" max="0" width="10" customWidth="1"/>`
- **THEN** no column 0 definition is created and the written `<cols>` block contains no `min="0"`

#### Scenario: Zero-based outline descriptor is ignored

- **WHEN** worksheet XML contains `<col min="0" max="0" outlineLevel="1"/>`
- **THEN** no column 0 definition is created

#### Scenario: Valid descriptors still parse

- **WHEN** worksheet XML contains `<col min="1" max="1" width="10" customWidth="1"/>`
- **THEN** column 1 is created with width `10`
