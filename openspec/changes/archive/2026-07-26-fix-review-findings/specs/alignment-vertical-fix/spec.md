## ADDED Requirements

### Requirement: AlignmentVertical::Middle Display emits OOXML-correct "center"

The `Display` impl for `AlignmentVertical::Middle` SHALL write `"center"` (matching OOXML §18.18.55 vertical alignment values) instead of `"middle"`.

#### Scenario: Display output

- **WHEN** calling `AlignmentVertical::Middle.to_string()`
- **THEN** the result SHALL be `"center"`

#### Scenario: Round-trip through writer

- **WHEN** emitting a style with `vertical: Some(AlignmentVertical::Middle)`
- **THEN** the writer SHALL produce `vertical="center"` in the XML

### Requirement: Reader maps OOXML "center" to AlignmentVertical::Middle

The reader SHALL map OOXML `vertical="center"` to `AlignmentVertical::Middle` (existing behavior, unchanged by this fix).

#### Scenario: Parse vertical center

- **WHEN** XML contains `vertical="center"`
- **THEN** the parsed alignment SHALL have `vertical = AlignmentVertical::Middle`

### Requirement: Writer has no special case for Middle/center

The writer SHALL NOT have a special-case override for `AlignmentVertical::Middle` — the generic `v.to_string()` SHALL produce the correct value.

#### Scenario: emit_alignment_child uses Display

- **WHEN** emitting alignment with `AlignmentVertical::Middle`
- **THEN** the code path SHALL use `v.to_string()` (not a hardcoded `"center"` override)
