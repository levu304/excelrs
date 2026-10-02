# Spec Delta

## REMOVED Requirements

### Requirement: excelrs maintains an ExcelJS feature-parity matrix

**Reason**: The requirement mandates the ongoing accuracy of a hand-maintained table in
`ROADMAP.md`. No validation can check it — CI runs structural OpenSpec validation, which does
not read `ROADMAP.md` — and its scenarios are all conditioned on the matrix being
"generated", a step no code or person performs. It has therefore decayed without detection,
and its closed status vocabulary (`shipped` / `partial` / `planned` / `n-a`) is already
violated by the `targeted` status the file uses in six rows. The v1.x porting program it
governs was declared complete at v2.0.0 and is preserved as a historical record by the
requirements retained in this capability.

**Migration**: The parity table remains in `ROADMAP.md` as reader-facing documentation of
what the library does. Its contents are no longer a specification-level invariant. Areas
still marked `planned` (charts, pivot tables, themes-write) are ordinary future feature work
and arrive as their own changes with their own capabilities, exactly as `formula-eval` did.
A reader needing the current behavior of an area is directed to `openspec/specs/*`, which is
CI-gated and is the rank-1 authority in `openspec/config.yaml`.

### Requirement: Parity matrix enumerates workbook, worksheet, and cell feature areas

**Reason**: The requirement's entire content is a hand-copied list of table rows that must
appear in `ROADMAP.md`. It cannot be validated, it goes stale silently, and it forces any
future editor of the document to consult a specification to learn which rows a document must
contain. This is the failure mode `spec-integrity`'s "Requirements state invariants, not
restated inventories" names directly.

**Migration**: None. The rows remain in `ROADMAP.md`; the requirement that they exist is
retired with the rest of the matrix contract.

### Requirement: Parity matrix enumerates styling and per-cell data feature areas

**Reason**: As above — a restated list of document rows with no possible validator.

**Migration**: None.

### Requirement: Parity matrix enumerates embedded-object and workbook-level feature areas

**Reason**: As above — a restated list of document rows with no possible validator.

**Migration**: None.

### Requirement: Parity matrix status is authoritative over the historical record

**Reason**: The requirement exists solely to order two artifacts — the live matrix and the
v2.0.0 completion record — against each other. With the matrix no longer a maintained
invariant, there is nothing for it to be authoritative over, and the v2.0.0 record is
retained on its own terms as a historical statement.

**Migration**: A reader determining the current behavior of a feature area reads
`openspec/specs/*` for that capability. The v2.0.0 record remains accurate as a statement
about what had shipped by that release.

### Requirement: Roadmap prioritizes unported features

**Reason**: Governs the derivation of an ordered porting roadmap from areas marked `partial`
or `planned`. The v1.x porting program was declared complete at v2.0.0; the remaining
unported areas (charts, pivot tables) are distant subsystems whose prioritization is a
product decision, not a standing derivation rule. Retaining it would keep mandating a
roadmap-generation step that no longer has an input.

**Migration**: Prioritization of the remaining areas (charts, pivot tables, whether to promote
`formula-eval` out of its opt-in Cargo feature) is tracked in `ROADMAP.md` and decided per
change. `ROADMAP.md` already records `formula-eval` promotion as an explicitly open product
decision.

### Requirement: Releases consume the roadmap and update the matrix

**Reason**: This is the requirement that failed silently. It mandates that a release update
its matrix row, and four rows still read `targeted` for features that shipped in v1.3.0 —
contradicting this capability's own scenario for v1.3.0. The requirement created an
obligation with no oracle, so its breach was invisible.

**Migration**: None. A change that ships behavior for a feature area updates
`CHANGELOG.md`, which is the rank-3 authority and is not subject to this failure.

### Requirement: Parity matrix rows reflect shipped behavior

**Reason**: Duplicates the obligation in the requirement above, restated as a correctness
claim rather than a process one. Retiring one without the other would leave the same
unenforceable obligation in place under a different name.

**Migration**: None.

### Requirement: excelrs preserves ExcelJS-compat getCell overloads

**Reason**: Misfiled, and factually false. It asserts the overloads "SHALL be re-injected
through a build-time hook" — a mechanism that has never functioned. `scripts/apply-glue.cjs`
gates its JavaScript branch on `basename === 'index.js'`, a file the build has never emitted,
so that branch cannot fire. The requirement's scenarios test only that the overloads resolve at
runtime, which passes because `index.js` is a hand-maintained file carrying a hand-pasted copy
— so the requirement read as satisfied while the mechanism it named was dead. The requirement
is also an FFI/build-pipeline guarantee with no relationship to ExcelJS feature parity.

**Migration**: The guarantee is restated under the `package-entrypoint` capability as
observable behavior — each `getCell` overload form resolves at runtime, the forms are declared
in the published type declarations, the entrypoint is a hand-maintained source that the build
does not overwrite, and CI asserts against the type declarations the build produces that the
cell-value union transforms are present.
