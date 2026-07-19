## ADDED Requirements

### Requirement: Formula-capture state resets at cell boundary

The streaming XLSX reader SHALL reset its formula-capture state at the end of every cell, so that a malformed or truncated cell missing its `</f>` end element cannot cause the next cell's value to be captured into the prior cell's formula.

#### Scenario: Missing `</f>` does not leak into the next cell

- **WHEN** a cell opens a `<f>` formula element but the corresponding `</f>` never arrives before the cell closes
- **THEN** the reader resets the formula-capture flag at the cell boundary, and the following cell's text/value is captured as that cell's own value (not appended to the prior cell's formula)
