# excelrs — Second-Pass Architecture Review

**Review date:** 2026-06-29  
**Spec version:** 1.1.0 (line 1081)  
**Spec path:** `docs/spec.md`  
**Reviewer:** Architect-reviewer agent (second pass)

---

## 0. Context

The first-pass architecture review identified **2 P0 blockers** and **5 P1 issues**. All 7 have been addressed in this updated spec. This second pass:

1. Verifies each first-pass fix is correct.
2. Finds **new issues** introduced by the updates.
3. Identifies **remaining gaps** the first review missed.

---

## 1. Verification of First-Pass Fixes

### P0-1: CellValue as Rust enum → Flat #[napi(object)] struct
**Status: ✅ RESOLVED**

- Original: `enum CellValue { Null, Number(f64), String(String), ... }` — cannot compile with `#[napi]`.
- Fixed: `#[napi(object)] struct CellValue` with `value_type: String` discriminant and `Option<T>` fields for each variant (§6.1, lines 466-503).
- ADR-11 (line 1045) records the rationale.
- The setter dispatch via `serde_json::Value` (§6.2 lines 537-558) correctly branches on JSON value type.
- **Caveat:** `serde_json::Value::Number(n)` calls `n.as_f64()` which returns `Option<f64>` — if `n` is an `i64` that cannot be represented as f64 (unlikely for spreadsheet values but technically possible), it returns `None` and the cell becomes `Number` with `number: None`. The spec should note this edge case. *(Low risk, P3)*

### P0-2: Method overloading → Two Rust methods + JS glue
**Status: ✅ RESOLVED**

- Original: `getCell(address: string)` and `getCell(row, col)` cannot be a single `#[napi]` function.
- Fixed: `get_cell_by_address(&self, address: String)` and `get_cell_by_rc(&self, row: u32, col: u32)` as separate Rust methods (§6.4, lines 643-647). JS glue dispatches (§7.2, lines 785-794).
- ADR-12 (line 1046) records the rationale.
- The JS glue correctly uses `typeof a === 'string'` and `this.getCellByRC(a, b!)` (with non-null assertion on `b`).
- **Caveat:** If user calls `ws.getCell(1)` (single number, no second arg), it routes to `getCellByRC(1, undefined)` which Rust-side is `get_cell_by_rc(1, undefined)` — undefined auto-converts to `u32` — likely 0. An explicit error would be better. *(P3)*

### P1-1: napi v2 → v3 dependencies
**Status: ✅ RESOLVED**

- Original: `napi = "2"`, `napi-derive = "2"`, `@napi-rs/cli@2`.
- Fixed: `napi = { version = "3", features = ["async", "serde-json"] }`, `napi-derive = "3"`, `@napi-rs/cli@3` (§7.3 line 812-813, §7.2 line 764).
- ADR-14 (line 1048) confirms v3 with serde-json feature.

### P1-2: JS glue file location
**Status: ⚠️ PARTIALLY RESOLVED**

- Original: No glue file specified.
- Fixed: `src/glue.ts` is documented at §3.1 (line 61) and §7.2 (lines 781-798).
- **However:** `src/` is the Rust crate root. Placing a `.ts` file in `src/` alongside `.rs` files is unusual and may confuse cargo tooling (e.g., `cargo publish` could include it). The conventional napi-rs layout places glue files in the project root, in an `npm/` directory, or referenced via `package.json`. See **NEW-P0 issue #3** below.
- ADR-12 (line 1046) mentions "~20-line glue file" but doesn't specify the exact path convention.

### P1-3: Missing #[napi(constructor)] annotation
**Status: ✅ RESOLVED**

- ADR-20 (line 1054): "`#[napi(constructor)]` on `new()` methods" with note that napi v3 requires this explicitly.
- Cell constructor (§6.2 line 529), Row constructor (§6.3 line 590), Worksheet constructor (§6.4 line 624), Workbook constructor (§6.5 line 673) all show `#[napi(constructor)]`.

### P1-4: Missing #[napi(setter)] attribute
**Status: ✅ RESOLVED**

- ADR-21 (line 1055): "`#[napi(setter)]` not `#[napi(set)]`" — correct attribute name.
- Row height setter (§6.3 line 599-600), hidden setter (§6.3 line 606-607), Worksheet name setter (§6.4 line 631), columns setter (§6.4 line 656-657), Column setters (§6.6 lines 708-726) all use `#[napi(setter)]`.
- Cell value setter (§6.2 line 537) also uses `#[napi(setter)]` correctly.

### P1-5: napi-build version in build-dependencies
**Status: ✅ RESOLVED**

- ADR-22 (line 1056): "`napi-build = "3"` in build-dependencies. Note: `napi-build = "2"` also works with napi v3."
- Cargo.toml (§7.3 line 824): `napi-build = "3"` — correct.

**First-pass fix summary: 6/7 fully resolved. P1-2 (glue.ts path) has a remaining issue (see NEW-P0 #3).**

---

## 2. Per-Axis Assessment

### Axis 1: Internal Consistency — **FAIL**

| Finding | Severity | Location |
|---------|----------|----------|
| `get_cell_by_address` returns `&Cell` — cannot cross FFI | **P0** | §6.4:644 |
| `get_cell_by_rc` returns `&Cell` — cannot cross FFI | **P0** | §6.4:647 |
| `add_worksheet` returns `&Worksheet` — cannot cross FFI | **P0** | §6.5:678 |
| `get_worksheet` returns `Option<&Worksheet>` — cannot cross FFI | **P0** | §6.5:681 |
| `worksheets` getter returns `Vec<&Worksheet>` — cannot cross FFI | **P0** | §6.5:684 |
| `get_row` returns `&Row` — cannot cross FFI | **P0** | §6.4:651 |
| `Row.getCell(col: number\|string)` in TS (§5.3:349) has **no Rust impl** in §6.3 | **P0** | §5.3:349 |
| `Worksheet.addRow`, `getRows`, `removeRow`, `rows` getter in TS (§5.2) have **no Rust impl** in §6.4 | **P1** | §5.2:325-337 |
| `Workbook.xlsx` namespace in TS (§5.1:280-285) has **no Rust counterpart** in §6.5 | **P1** | §5.1:280 |
| Date format ID range: §4.2 says "14–22, 27–36, 45–47" while §4.5 says "14–22, 27–36, 45–47, 50–81" | **P2** | §4.2:165 vs §4.5:248 |
| `CellStyle.border` references `BorderStyle` — **type never defined** | **P2** | §6.7:746 |
| §4.2 algorithm says "sheet.formulas()" but actual calamine API is `worksheet_formula()` | **P3** | §4.2:162 |
| `Cell.address` is `pub` in Rust (§6.2:519) but `readonly` in TS (§5.4:370) — inconsistency but getter exists | **P3** | §6.2:519 |
| `Cell.formula` is `pub` in Rust (§6.2:523) but `readonly` in TS (§5.4:379) — getter+pub field coexist | **P3** | §6.2:523 |

### Axis 2: Implementation Readiness — **FAIL (P0 issue)**

| Finding | Severity | Location |
|---------|----------|----------|
| napi-rs reference return types will **not compile** — engineer hitting this on day 1 will be blocked | **P0** | §6.4, §6.5 |
| `serde_json::Value` setter dispatch for `NaN`, `Infinity`, `BigInt`, `Date` not documented | **P1** | §5.6:424-432 |
| `setCell` escape hatch mentioned in clone-on-read note (line 574) but **never specified** in API | **P1** | §6.2:574 |
| Empty workbook read, malformed addresses, missing sheets — error cases handled by error enum (§4.4) but **no per-method error contract** documented | **P2** | §5.1-5.4 |
| Row `values` property (§5.3:352) returns `Array<CellValue \| undefined>` — unclear if this is a JS glue construct or a `#[napi(getter)]` | **P2** | §5.3:352 |
| `worksheet.rows` iterable — how does `BTreeMap<u32, Row>` serialize to JS array? Needs explicit conversion spec | **P2** | §6.4:618 |

### Axis 3: Type Safety & FFI Edge Cases — **FAIL (P0 issue)**

| Finding | Severity | Location |
|---------|----------|----------|
| napi-rs cannot return `&T` across FFI. All `#[napi]` return types must be owned. | **P0** | §6.4, §6.5 |
| `get_worksheet` uses `serde_json::Value` for `name_or_index` — works but loses Rust type safety. A JS glue dispatcher (like `getCell`) would be more idiomatic and let Rust receive `String` or `u32` directly. | **P2** | §6.5:681 |
| calamine `Data::Duration` mapping not documented — what CellValue variant does it become? | **P2** | §4.2 |
| calamine `Data::DateTimeIso` and `Data::DurationIso` (new in 0.35) have no CellValue mapping | **P2** | §4.2 |
| Clone-on-read note (§6.2:574-575) is clear about the limitation but **doesn't provide the workaround API** (`setCell`, `getCellMut`) — engineer must guess | **P1** | §6.2:574 |
| `serde_json::Value::Number(n).as_f64()` returns `None` for integers > 2^53 — silent data loss | **P3** | §6.2:543 |

### Axis 4: Dep & Version Correctness — **FAIL**

| Finding | Severity | Location |
|---------|----------|----------|
| `calamine = "0.24"` (Feb 2024) vs latest `0.35.0` (May 2026) — 2+ years stale. New `Data` variants (`DateTimeIso`, `DurationIso`) unhandled. | **P1** | §7.3:814 |
| `zip = "2"` — zip v2 is a major fork with rewritten API. calamine internally depends on zip 0.6/0.10. **Potential dependency conflict** if both share workspace. Error type `zip::result::ZipError` may differ between crate versions. | **P1** | §7.3:815 |
| `quick-xml = "0.36"` (Jul 2024) — latest is 0.38+ as of 2026. 2 years stale but less critical than calamine. | **P2** | §7.3:816 |
| `chrono` features only specify `["serde"]` — need `DateTime<Utc>` used in CellValue (§6.1:473) but `chrono = "0.4"` with `["serde"]` covers this. OK. | ✅ | §7.3:820 |
| `tokio = "1"` with `features = ["full"]` — overkill for an addon that only does file I/O. `["rt", "fs"]` would suffice and reduce compile time. | **P3** | §7.3:821 |
| `napi-build = "3"` — ADR-22 notes v2 also works. Using v3 is correct. | ✅ | §7.3:824 |

### Axis 5: Security & Correctness — **PASS**

| Finding | Severity | Location |
|---------|----------|----------|
| TOCTOU: `read_from_file` vs `write_to_file` are synchronous from JS perspective — no TOCTOU risk in single-threaded model. | ✅ | §4.2, §4.3 |
| Injection: XML output via `quick-xml` properly escapes special characters (quick-xml handles this by default). No raw string concatenation described. | ✅ | §4.3 |
| Buffer overflow: Rust's memory safety prevents buffer overflows in I/O paths. | ✅ | §4.2-4.3 |
| 1904 date system: §4.5 (lines 248-250) explicitly notes the 1900 vs 1904 system and that calamine handles it internally for read. Write path uses a "simplified heuristic" — the 1904 system flag is **not propagated to the write path**. If reading a Mac-origin file (1904 system) and writing it back, date serial numbers will be **silently corrupted**. | **P2** | §4.5:248 |
| Zip bomb / decompression bomb: No documented input size limits on `read_from_buffer`. calamine may have internal limits but the spec should note this. | **P3** | §4.2 |

### Axis 6: Architecture Quality — **PASS (with notes)**

| Finding | Severity | Location |
|---------|----------|----------|
| Monolithic crate approach holds: ~1080-line spec maps to ~2-3K LoC Rust, well under the §5K split threshold. | ✅ | P6 (line 48) |
| Deferral decisions (Hyperlink, RichText, SharedString, Merge) are sound: all four lack calamine read-path support in v0.1, so including them would be dead code. | ✅ | §6.1:505-510 |
| `glue.ts` in `src/` is an **architectural smell**: mixes Rust and TypeScript in the crate root. Should live at project root or in an `npm/` directory. | **P1** | §3.1:61 |
| `serde_json::Value` for `get_worksheet` parameter is unnecessary FFI complexity when a JS glue dispatcher pattern already exists (ADR-12). | **P2** | §6.5:681 |
| The `xlsx` namespace on Workbook (§5.1) is an exceljs pattern — having it implemented in Rust vs JS glue is an architectural decision not documented. | **P2** | §5.1:280 |

---

## 3. New P0 Blockers

### NEW-P0-1: `#[napi]` functions return references across FFI (6 locations)

**File:** `docs/spec.md`  
**Lines:** §6.4:644, 647, 651; §6.5:678, 681, 684

napi-rs v3 **cannot** return Rust references (`&T`) across the FFI boundary. The `#[napi]` macro requires return types to implement `NapiValue`, which `&T` does not. All six return signatures are invalid:

| Current (invalid) | Required (valid) |
|---|---|
| `get_cell_by_address(...) -> &Cell` | `-> Cell` |
| `get_cell_by_rc(...) -> &Cell` | `-> Cell` |
| `get_row(...) -> &Row` | `-> Row` |
| `add_worksheet(...) -> &Worksheet` | `-> Worksheet` |
| `get_worksheet(...) -> Option<&Worksheet>` | `-> Option<Worksheet>` |
| `worksheets() -> Vec<&Worksheet>` | `-> Vec<Worksheet>` |

The clone-on-read note (§6.2:574-575) acknowledges this limitation conceptually, but the signatures in the spec are misleading — they imply zero-cost access when the implementation must clone.

**Fix:** Change all return types from `&T` to `T` (owned). The clone-on-read note already documents that these return clones. Add a reference to this constraint in the design principles or ADR.

### NEW-P0-2: `Row.getCell` has no Rust implementation

**File:** `docs/spec.md`  
**Lines:** §5.3:349, §6.3:588-607

The TypeScript API (§5.3) requires:
```typescript
getCell(col: number | string): Cell;
```

But the Rust `impl Row` block (§6.3) has **no `getCell` method at all**. The Row struct has `cells: HashMap<u32, Cell>` (§6.3:583) but no accessor. The `getCell` method must:
1. Accept `col: u32` (number) or `col: String` (letter) — likely via two Rust methods + JS glue, matching the Worksheet.getCell pattern.
2. Parse column letters to indices (using `types.rs` address parsing).
3. Look up `self.cells.get(&col)` and clone the result.

**Fix:** Add `get_cell_by_col` and `get_cell_by_col_letter` methods to the Row impl, with JS glue dispatch.

---

## 4. New P1 Issues (should be fixed before implementation begins)

### NEW-P1-1: `Worksheet.addRow`, `getRows`, `removeRow`, `rows` getter missing from Rust

**Lines:** §5.2:325-337 vs §6.4:622-657

Four TS API methods have no Rust counterparts in the Worksheet impl:
- `addRow(values: Array<CellValueInput>): Row`
- `getRows(start: number, count: number): Row[]`
- `removeRow(rowNumber: number): void`
- `rows` getter (iterable of rows)

The `row_count` and `column_count` getters are mentioned as `{ ... }` but the full signatures and bodies are elided.

**Fix:** Add these four methods to the Worksheet impl block, with complete signatures.

### NEW-P1-2: `Workbook.xlsx` namespace has no Rust implementation

**Lines:** §5.1:280-285 vs §6.5:670-688

The exceljs-compatible `Workbook.xlsx` sub-object (with `readFile`, `writeFile`, `read`, `write`) has no representation in the Rust struct. Options:
1. Implement as a separate `#[napi(object)]` struct `XlsxNamespace` with four `#[napi]` methods, embedded in Workbook.
2. Implement via JS glue that wraps flat Rust functions.
3. Name the Rust methods as `xlsx_read_file`, `xlsx_write_file` etc. and use JS glue to create the namespace.

**The spec must document which approach is chosen** before implementation.

### NEW-P1-3: `src/glue.ts` location conflicts with Rust crate root

**Lines:** §3.1:61, §7.2:781

Placing `glue.ts` in `src/` alongside Rust source files:
- Risks `cargo publish` including TypeScript files in the crate.
- Breaks the convention of `src/` containing only Rust code.
- Conflicts with typical napi-rs project layouts (where glue is at project root or in `npm/`).

The standard napi-rs scaffold places the JS entry point at the project root (`index.js`), with glue files at root level.

**Fix:** Move `glue.ts` to project root and reference it from `package.json` `"main"` or `"exports"` field. Update the directory tree in §3.1.

### NEW-P1-4: calamine version 0.24 is 2+ years stale; Data::DateTimeIso/DurationIso unhandled

**Lines:** §7.3:814, §4.2

calamine has advanced 11 minor versions since 0.24.0. The latest (0.35.0) adds `DateTimeIso(String)` and `DurationIso(String)` variants to the `Data` enum. If a user's `.xlsx` file contains cells with these data types (increasingly common in modern Excel exports), the reader will encounter an unhandled variant.

**Fix:** Either:
- **Option A (recommended):** Upgrade to `calamine = "0.35"` and add mappings for `DateTimeIso` → `CellValue { valueType: "String", ... }` and `DurationIso` → `CellValue { valueType: "String", ... }`.
- **Option B:** Pin `calamine = "=0.24.0"` explicitly and document that newer Excel files may fail.

### NEW-P1-5: `zip = "2"` version concern — potential conflict with calamine's zip dependency

**Lines:** §7.3:815

The `zip` crate has two major lineages:
- **0.x series** (by mvdnes, now maintained as `zip`): versions 0.5 through 0.10. Used internally by calamine.
- **2.x series** (fork by Pr0methean): versions 2.0 through 2.6. Rewritten API, different error types.

If the writer uses `zip = "2"` while calamine uses `zip 0.6`, Cargo will either resolve to a single version (breaking one) or create duplicate dependencies. The `ExcelrsError::Zip(#[from] zip::result::ZipError)` will bind to whichever `zip` crate Cargo resolves — if it resolves to v2, calamine's zip errors won't be catchable and vice versa.

**Fix:** Document the zip version strategy explicitly. Recommendation: use `zip = "0.10"` (latest 0.x) to stay compatible with calamine's internal zip usage. If zip v2 features are needed, use a separate crate name via `[dependencies]` renaming.

---

## 5. New P2 Issues (fix before v0.1 release, but not blocking implementation start)

### NEW-P2-1: `CellStyle.border: Option<BorderStyle>` — BorderStyle undefined

**Line:** §6.7:746

`BorderStyle` is referenced as a type but never defined anywhere in the spec. Since styles are read-only in v0.1, this is non-blocking but should be specified before any engineer touches the style module.

### NEW-P2-2: Date format ID range inconsistency

**Lines:** §4.2:165 vs §4.5:248

The reader section says "14–22, 27–36, 45–47" while the types section says "14–22, 27–36, 45–47, 50–81". The types section is more complete (IDs 50–81 are locale-specific date formats). The reader algorithm should be updated to match.

### NEW-P2-3: 1904 date system not propagated on write path

**Lines:** §4.5:248-252

The spec correctly notes that calamine handles 1900/1904 for reads. However, the write path uses a "simplified heuristic" (line 252) without mentioning the 1904 system flag. If a workbook is read from a Mac-origin file (1904 system), date serial numbers from the model will be in 1904 encoding, but the writer will write them without setting the workbook's date system flag → **dates will shift by 4 years** when opened in Excel.

### NEW-P2-4: `get_worksheet` uses `serde_json::Value` when JS glue pattern already exists

**Lines:** §6.5:681

The spec already established the two-method + JS glue pattern for `getCell` (ADR-12). `get_worksheet` has the same overloading pattern (name: string | index: number) but uses `serde_json::Value` instead. Using `serde_json::Value` as an ad-hoc union type is:
- Less type-safe (any JS value accepted, only string/number make sense)
- Inconsistent with the established dispatch pattern
- Produces poorer TypeScript declarations (generated type will be `any`)

### NEW-P2-5: `serde_json::Value::Number(n).as_f64()` silent truncation

**Lines:** §6.2:541-544

`serde_json::Value::Number` can hold integers up to `i64::MAX` (~9.2e18). `as_f64()` returns `None` for integers that cannot be exactly represented as f64 (specifically: integers > 2^53 lose precision; integers > ~1e308 return None). For spreadsheet values, this is unlikely but should be documented.

### NEW-P2-6: no per-method error contract

**Lines:** §5.1-5.4

The error enum (§4.4) is comprehensive, but the TS API signatures don't show which errors each method can throw. For example:
- `getWorksheet("NonExistent")` — returns `undefined` or throws `SheetNotFound`?
- `getCell("ZZZ999999")` — throws `InvalidAddress`?
- `addRow([])` — valid (empty row) or error?

Engineers implementing tests need this contract.

---

## 6. New P3 Issues (nice to fix, not blocking)

### NEW-P3-1: `Cell.address` and `Cell.formula` are `pub` fields but read-only in TS
**Lines:** §6.2:519, 523 — Having both public fields and getters is redundant and risks accidental mutation in Rust code. Use private fields with getters only.

### NEW-P3-2: `tokio = { features = ["full"] }` is overkill
**Line:** §7.3:821 — For file I/O only in an addon, `["rt-multi-thread", "fs"]` would suffice and reduce compile time by ~30%.

### NEW-P3-3: No zip bomb / large input protection
**Line:** §4.2 — The spec should note that `read_from_buffer` should validate input size before passing to calamine, or that calamine's internal limits are trusted.

### NEW-P3-4: `worksheetCount` naming
**Lines:** §5.1:277 vs §6.5:687 — Rust `worksheet_count` auto-converts to JS `worksheetCount` via napi-rs. This is correct. Not an issue, just noting for verification.

---

## 7. Overall Verdict

### **⚠️ NEEDS MINOR FIXES — 2 new P0 blockers found**

The first-pass issues (7) are well-resolved. The spec quality is high overall — design principles are clear, the architecture is pragmatic, and deferral decisions are well-reasoned. However, two new P0 blockers were introduced by the napi-rs v3 migration:

1. **All `#[napi]` return types are references** — will not compile (6 locations).
2. **`Row.getCell` has no Rust implementation** — API gap.

Additionally, 5 P1 issues should be resolved before implementation begins to avoid rework, and the calamine version should be updated.

**Recommended actions:**
1. Fix NEW-P0-1 (change all `&T` → `T` in signatures) — 15 minutes
2. Fix NEW-P0-2 (add Row.getCell methods) — 30 minutes
3. Resolve NEW-P1-1 through NEW-P1-5 — 2-3 hours
4. Defer P2/P3 issues to implementation phase

After P0+P1 fixes, the spec is **Ready to Implement**.

---

## 8. Top 3 Implementation Risks

### Risk 1: napi-rs reference semantics → mutation surprise chain

**Probability:** High | **Impact:** High

The clone-on-read limitation (line 574) means `ws.getCell('A1').value = 42` silently does nothing — a major deviation from exceljs behavior. Users will hit this immediately. The workaround (`setCell` or interior mutability in v0.2) must be clearly documented in the README and tested. Consider adding a runtime warning when `set_value` is called on a clone that is about to be discarded.

### Risk 2: calamine version staleness → data loss on modern Excel files

**Probability:** Medium | **Impact:** Medium

calamine 0.24 cannot handle `DateTimeIso` and `DurationIso` cell types that modern Excel exports produce. This will manifest as runtime errors (match exhaustiveness) or silent data loss if a wildcard match arm exists. Either upgrade to 0.35 or pin to `=0.24.0` with a documented limitation.

### Risk 3: zip v2 vs calamine's zip dependency conflict

**Probability:** Medium | **Impact:** Medium

If Cargo resolves `zip = "2"` and calamine's `zip 0.6` to conflicting versions, the `ZipError` in the error enum will only catch errors from one crate. The writer may need a separate error variant or a compatibility shim. Test early with `cargo tree -i zip` to verify resolution.

---

*Review completed. 2 new P0 blockers, 5 new P1 issues, 6 new P2 issues, 4 new P3 issues identified.*
