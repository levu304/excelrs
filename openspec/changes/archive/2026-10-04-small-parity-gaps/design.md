# Design

## Context

See proposal.md (Why) and the three delta specs for required behavior. Current state shaping the approach:

- `Column` (`src/model/column.rs`) is a plain struct; `Worksheet.columns()` returns detached clones, so handle mutations go nowhere. `Row`/`Cell` already solve the FFI-clone problem with interior mutability, and `getCell` overloads are split as `getCellByAddress`/`getCellByRc` in Rust with the overload unified in hand-maintained `index.js` glue (`#[napi]` cannot express overloads).
- `emit_color_attrs` (`src/writer/styles.rs`) prefers resolved rgb whenever present, and unit tests pin that policy. The reader already populates both rgb and the theme link (`Color { rgb, theme, tint }`); the writer simply never uses the link. The writer never emits `xl/theme/theme1.xml`.
- The in-memory reader delegates formulas to calamine's `worksheet_formula()`, which returns bare strings — `t`/`ref`/`si` attributes never reach excelrs. The streaming path (`src/stream.rs`) already parses `<f>` attributes but only branches on `shared`.
- `addRow` takes positional arrays only, so `Column.key` has no in-repo consumer. `columnCount` derives from row data, not the columns vec.

## Goals / Non-Goals

**Goals:**

- `getColumn` behaves like `getRow`: live handle, auto-create, mutations persist across the FFI boundary.
- Themed files round-trip without semantic downgrade; rgb-only files emit byte-identical output.
- Array-formula behavior is specified and pinned by test, with the deferral rationale recorded where a future reader will find it.

**Non-Goals:**

- Key-based column lookup, `spliceColumns`, `eachColumn`, keyed-object rows — no consumer exists; each would be speculative surface.
- Themed-color authoring API (`color_theme`/`tint` stay `#[napi(skip)]`).
- Array preservation or spill evaluation.

## Decisions

### D1: Column becomes a live handle via interior mutability (Row precedent)

Give `Column`'s mutable fields the same shared-ownership treatment `Row`/`Cell` already use, so a `Column` cloned across the FFI boundary writes through to the worksheet model. `setColumns` keeps total-replace semantics; outstanding handles from before a replace detach (same class of behavior as `duplicateRow`'s style detach), while fresh `getColumn` lookups always resolve against the current vec.

- Rejected: snapshot copy. `ws.getColumn(1).width = 20` silently doing nothing repeats the exact trap `setColumns`-only sets today.
- Rejected: index-proxy without shared state. Breaks the moment `setColumns` reorders or a sparse `colNum` shifts positions; the Row precedent is tested and understood.

### D2: Rust split methods plus JS-glue overload for getColumn

Expose `getColumnByNum` / `getColumnByLetter` from Rust and unify as `getColumn` in `index.js`, mirroring `getCellByAddress`/`getCellByRc` → `getCell`. Letter parsing reuses the existing column-letter conversion.

- Rejected: single untyped parameter. The `setColumns` type-safety change established that untyped FFI parameters rot into runtime errors; two typed entry points keep `native.d.ts` honest and the glue thin.

### D3: Theme-wins emission plus verbatim theme1.xml passthrough

Flip `emit_color_attrs` to prefer the theme reference when a theme link is present (rgb only when no link exists), store the raw `theme1.xml` bytes on read, and emit them verbatim on write with the required content-type entry. Verbatim passthrough (never re-serialize from the parsed scheme) preserves custom extensions the parser does not model.

- Rejected: keep rgb-wins. Visually correct but semantically lossy — Excel theme-switching stops affecting converted cells, and the conversion is silent.
- Rejected: authoring API. New public FFI surface for a writer nobody in-repo can call; a separate change if demand appears.
- Rejected: re-resolving rgb back to theme on write. A guess — tint makes the mapping ambiguous — where the reader already hands us the true link.
- Supersedes: the rgb-wins policy pinned by the `writer/styles.rs` theme tests, which this change updates.

### D4: Array formulas specified as degrade, not preserved

Probe evidence (ExcelJS-authored `t="array"` file, three carrying cells): every cell reads as `Formula` with text and cached value intact, and writes back as plain `<f>` + `<v>`. No crash, no repair prompt, no value loss — only recalc-time array semantics are dropped. Preservation would need a second `<f>`-attribute ingestion path beside calamine plus cell-model carriage for `t`/`ref`, for semantics that matter only at Excel-recalc-time on legacy CSE files. The spec records the degrade contract and a fixture test pins it.

- Rejected: preserve-opaque now. Cost (new ingestion path, model fields, writer branches) against a latent-only failure on a legacy encoding.
- Revisit trigger: a real fixture whose post-round-trip Excel recalculation diverges from the cached values.

## Risks / Trade-offs

- [Risk] Existing `writer/styles.rs` theme tests pin rgb-wins and will fail → Mitigation: tasks update them to theme-wins first (policy flip is the point, not collateral).
- [Risk] Missing content-type entry for a newly emitted theme part produces a corrupt package → Mitigation: tasks include an OOXML-validity assertion on a custom-theme round-trip (part present, content-type present, reference resolves).
- [Risk] `setColumns`-after-`getColumn` detaches outstanding handles → Mitigation: documented in design (D1) and covered by a scenario-free task verification (fresh lookup sees replaced definition); matches established detach precedent.
- [Risk] `columnCount` still derives from row data, so a width-only `getColumn` column does not move `columnCount` → Mitigation: explicitly unchanged; ExcelJS parity on that getter is not part of this change (no spec claims it).
- [Risk] Style-table dedup keys on serialized structs where theme links are `#[serde(skip)]`, so a themed entry and a plain entry resolving to the same rgb merge first-wins → Mitigation: accepted; pure themed files (the passthrough case) carry links on every twin-free entry, and collisions need an identically-valued plain twin in the same workbook
- [Risk] Glue-pipeline drift (`apply-glue.cjs`, `verify-build-output.cjs`) when declarations change shape → Mitigation: tasks run `pnpm verify:build` inside the relevant group, not in a final group.

## Migration Plan

No migration. Additive API (`getColumn`), policy flip confined to files carrying theme references (rgb-only output byte-identical), array behavior unchanged in code (newly specified and tested). Rollback is revert; no data migration exists.

## Open Questions

- Unparseable letter input to `getColumn` (e.g. `"!!"`): napi error vs. empty-handle. Recommendation: napi error (`getCellByAddress`'s empty-cell fallback exists because cell reads must be total; a column lookup has no address to return). Answerable during implementation without changing specs or tasks.
