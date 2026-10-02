## REMOVED Requirements

### Requirement: writer emits `<v>` for each cached scalar on Formula cells

**Reason**: Bundled two independent behaviors (cached-value emission and `t`-attribute priority) into one requirement whose prose exceeded the 500-character guideline.

**Migration**: Replaced by "Writer emits `<v>` for each cached scalar on Formula cells", "Writer sets the Formula cell `t` attribute by cached-scalar priority", and "Writer emits a `date_serial` cached branch on Formula cells".

### Requirement: Cell.cachedValue is recalc-only

**Reason**: Combined the observable getter contract with the internal `recalc_only` flag implementation detail, producing over-long prose.

**Migration**: Replaced by "Cell.cachedValue is recalc-only" (observable contract) and "Cached values from authoring and reader paths are not exposed as recalc results".

## ADDED Requirements

### Requirement: Writer emits `<f>` and `<v>` for each cached scalar on Formula cells

When a `Formula` `CellValue` carries a cached scalar, the writer SHALL emit
`<f>{formula}</f><v>{cached}</v>`.

#### Scenario: Formula cell with a cached scalar emits both elements

- **WHEN** the writer serializes a `Formula` `CellValue` whose cached field is populated
- **THEN** it SHALL emit `<f>{formula}</f>` followed by `<v>{cached}</v>` for that cell

#### Scenario: Formula cell without a cached scalar emits no `<v>`

- **WHEN** the writer serializes a `Formula` `CellValue` whose cached field is absent
- **THEN** it SHALL emit `<f>{formula}</f>` and no `<v>` element for that cell

### Requirement: Writer sets the Formula cell `t` attribute by cached-scalar priority

The `t` attribute emitted for the `Formula` arm SHALL check cached scalar fields in the
priority order `number → string → boolean → error_value → date_serial`, matching the order
the value-writing arm uses. `number` and `date_serial` SHALL produce no `t` attribute.
Precedence SHALL prevent a `t` attribute that contradicts the emitted `<v>` content.

#### Scenario: Boolean cached value wins over a simultaneously present string

- **WHEN** a `Formula` `CellValue` carries both a `boolean` and a `string` cached field
- **THEN** the emitted `t` SHALL be `"b"` and the `<v>` SHALL contain the boolean's value

#### Scenario: Numeric cached value emits no type attribute

- **WHEN** a `Formula` `CellValue` carries a `number` cached field
- **THEN** the cell SHALL be emitted with no `t` attribute and `<v>` containing the number

### Requirement: Writer emits a `date_serial` cached branch on Formula cells

The `Formula` arm of the writer SHALL emit a `date_serial` cached branch, mirroring the
`Date` arm, so a cached date scalar round-trips through the `<v>` element.

#### Scenario: Cached date serial on a Formula cell

- **WHEN** the writer serializes a `Formula` `CellValue` whose cached scalar is a `date_serial`
- **THEN** it SHALL emit the date serial as the `<v>` content with no `t` attribute

### Requirement: Cell.cachedValue exposes only recalculated cached scalars

The `Cell.cachedValue` getter SHALL return a cached scalar only when the cell was evaluated
via `Worksheet::recalculate()`. For every other state the getter SHALL return `null`.

#### Scenario: Recalculated cell exposes its cached value

- **WHEN** `Worksheet::recalculate()` evaluates a `Formula`-typed cell
- **THEN** `Cell.cachedValue` SHALL return the evaluated cached scalar

#### Scenario: Authored cell does not expose a cached value

- **WHEN** a `Formula`-typed cell has a cached scalar set through the reader or the JS value setter but has not been recalculated
- **THEN** `Cell.cachedValue` SHALL return `null`

### Requirement: Cached values from authoring and reader paths are not exposed as recalc results

A cached scalar set through the reader's raw cell-value and cell-formula insertion paths, or
through the JS value setter, SHALL NOT be observable through `Cell.cachedValue`. Only the
recalculation path SHALL make a cached scalar observable there.

#### Scenario: Reader-inserted cached scalar is hidden

- **WHEN** the reader inserts a `Formula` cell carrying a cached scalar
- **THEN** `Cell.cachedValue` SHALL return `null` until recalculation runs

#### Scenario: JS setter cached scalar is hidden

- **WHEN** a JS caller sets a `Formula` cell's value carrying a cached scalar
- **THEN** `Cell.cachedValue` SHALL return `null` until recalculation runs