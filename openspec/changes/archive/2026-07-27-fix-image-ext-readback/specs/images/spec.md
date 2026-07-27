## MODIFIED Requirements

### Requirement: Round-trip preserves anchor

The `anchor` read back by `getImages()` SHALL preserve the original shape
(top-left `tl`, and `br` for two-cell or `ext` for one-cell) and its values,
within float precision, across a write → read → write cycle.

For a one-cell anchor with explicit `ext` (`{ tl, ext: { width, height } }`),
the pixel size SHALL be preserved through read and write: the emitted
`<xdr:ext cx="W*9525" cy="H*9525"/>` SHALL be parsed on read and returned as
`ext: { width, height }` by `getImages()`.

#### Scenario: One-cell anchor size survives read-back

- **WHEN** an xlsx written with `anchor: { tl: { col: 1, row: 1 }, ext: { width: 120, height: 60 } }` is read back
- **THEN** `getImages()[0].anchor` reports `tl: { col: 1, row: 1 }` and `ext: { width: 120, height: 60 }` (rounded to 4 dp), and re-writing emits `<xdr:ext cx="1143000" cy="571500"/>`.

#### Scenario: getImages returns valid shape for one-cell read from file

- **WHEN** a `oneCellAnchor` with `<xdr:ext>` is read from a file
- **THEN** `getImages()[0].anchor` has `ext` set and `br` unset (valid per the "exactly one of br/ext" contract), never `br: None, ext: None`.

### Requirement: Writer embeds media and emits drawing part

A worksheet with images SHALL emit a drawing part containing one
`<xdr:oneCellAnchor>` or `<xdr:twoCellAnchor>` per image. A `oneCellAnchor` SHALL
always contain a `<xdr:ext cx=".." cy=".."/>` child (ECMA-376 20.5.2.27) — even
when the internal `ext_size` is absent — so the output is schema-valid and
openable by Excel. A `twoCellAnchor` SHALL contain a `<xdr:to>` child and no
`<xdr:ext>`.

#### Scenario: Writer emits ext for one-cell even without explicit size

- **WHEN** a `oneCellAnchor` is written with `ext_size: None`
- **THEN** the emitted XML contains `<xdr:ext cx="0" cy="0"/>` (fallback), so the file remains valid OOXML.
