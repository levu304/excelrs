## Why

`excelrs` advertises drop-in [ExcelJS](https://github.com/exceljs/exceljs) compatibility as its core design principle, but the governing roadmap (`docs/spec.md` §9.2.1) was last reconciled at v0.6.0 and no longer reflects what actually shipped in v0.7–v0.9 (defined names, data validation, CSV read/write). There is no current, evidence-based map of *where excelrs stands against ExcelJS today* or *what to port next*. Without it, future releases are picking features ad hoc instead of closing the compatibility gap deliberately. v0.10.0 should fix that: explore ExcelJS's real API surface, measure parity against shipped excelrs code, and align a prioritized, release-sequenced porting roadmap.

## What Changes

- A research/planning change — **no runtime code, no public-API changes, no shipped behavior**.
- Introduces the `exceljs-parity` capability: a tracked, versioned parity matrix + porting roadmap that future releases consume and update.
- Produces a concrete deliverable: `ROADMAP.md` mapping each ExcelJS feature area to `shipped` / `partial` / `planned` / `n-a`, with a prioritized sequence of target releases (v0.11.0+).
- Reconciles the stale `docs/spec.md` §9.2.1 roadmap against code that has actually shipped.

## Capabilities

### New Capabilities

- `exceljs-parity`: Tracks excelrs's ExcelJS feature-parity status and prioritizes the porting roadmap. Each future release MODIFIES this spec to record its new `shipped`/`partial` status.

### Modified Capabilities
<!-- No existing capability's requirements change in this planning change. -->

## Impact

- **Code**: None. Pure planning change.
- **Artifacts**: New `openspec/specs/exceljs-parity/spec.md`, `ROADMAP.md` at repo root, updated `design.md`/`tasks.md` in this change.
- **Process**: Future releases (v0.11.0+) are expected to consume the next roadmap item and update `exceljs-parity`.
- **Dependencies**: None added. Research uses publicly available ExcelJS docs/source.
