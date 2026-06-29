# Architecture Review — excelrs spec v1.0.0

**Date:** 2026-06-29  
**Reviewer:** Architecture review (pre-implementation gate)  
**Spec:** `docs/spec.md` (836 lines)  
**Verdict:** ⚠️ **Not ready for implementation** — 2 P0 blockers, 5 P1 issues, 8 P2 gaps. P0 items must be resolved before any code is written.

---

## Summary of findings

| Severity | Count | Summary |
|----------|-------|---------|
| 🔴 P0 — Blocker | 2 | Spec claims about napi-rs capabilities are factually incorrect; implementation would discover these at the Rust level and require re-architecture |
| 🟡 P1 — Important | 5 | Missing OOXML edge cases, calamine type-system gaps, and design decisions that will cause bugs or rework |
| 🟢 P2 — Nice-to-have | 8 | Scope realism, testing strategy, and future-proofing gaps |

---

## 1. Architectural Soundness (P1–P2)

### ✅ What works

The module structure (`model/`, `reader/`, `writer/`, `error.rs`, `types.rs`) is **well-chosen** for a spreadsheet library. It cleanly separates concerns:
- `model` as single source of truth (lines 126–143) — correct instinct
- `reader` as translation layer from calamine types (lines 147–168) — correct
- `writer` as model-to-OOXML serializer via quick-xml (lines 170–194) — correct
- Monolithic crate decision (P6, line 46–48) — correct for a solo dev at ~1–2K LoC

The dependency rationale (lines 110–118) is sound. calamine for read is the right call. zip + quick-xml for write gives necessary control.

### 🟡 P1 — Hidden coupling point: calamine Data ↔ CellValue mapping is not 1:1

**Spec reference:** Lines 156–159, lines 449–462

calamine's `Data` enum and excelrs's proposed `CellValue` enum have **fundamentally different variant sets**:

| calamine `Data` | excelrs `CellValue` | Mapping |
|---|---|---|
| `Empty` | `Null` | Direct ✅ |
| `Bool(bool)` | `Boolean(bool)` | Direct ✅ |
| `Float(f64)` | `Number(f64)` | Merge into one ✅ |
| `Int(i64)` | (same as Number) | Merge into Number ✅ |
| `String(String)` | `String(String)` | Direct ✅ |
| `DateTime(ExcelDateTime)` | `Date(DateTime<Utc>)` | Conversion needed ✅ |
| `DateTimeIso(String)` | — | **No excelrs variant** ⚠️ |
| `DurationIso(String)` | — | **No excelrs variant** ⚠️ |
| `Error(CellErrorType)` | `Error(String)` | Conversion needed ✅ |
| — | `Formula(String)` | **Not in calamine cell data** 🔴 |
| — | `Hyperlink(String)` | **Not in calamine cell data** 🔴 |
| — | `RichText(Vec<RichTextRun>)` | **Not in calamine cell data** 🔴 |
| — | `SharedString(String)` | **calamine resolves these** 🔴 |
| — | `Merge` | **Not in calamine** 🔴 |

**Impact:** 5 of 11 CellValue variants have no calamine source. The spec's reader section (lines 164–167) says "Map calamine `Data` enum → `CellValue` enum" but doesn't acknowledge that calamine stores formulas in a **separate API** (`worksheet_formula()`), doesn't parse inline rich text (`<is><r>`), and doesn't expose hyperlinks in the cell data stream. The spec must either:

1. Document that Formula/Hyperlink/RichText/Merge will be `Null` on read in v0.1, or
2. Extend the reader with separate calamine API calls (formula + hyperlink), or
3. Drop those variants from v0.1 and reintroduce them when the reader supports them.

**Recommendation:** Explicitly mark Formula, Hyperlink, RichText, and Merge as **write-only in v0.1** (populated from JS, preserved through write, always Null on read). This is a 2-line addition to lines 164–168.

### ✅ Monolithic crate scales to v0.2

The "split at ~5K LoC" heuristic (line 47) is reasonable. The v0.2 style system would be ~1–2K additional lines. A single crate handles this fine. The real split trigger would be if someone wants to use the Rust core without napi-rs bindings (e.g., a CLI tool), which would motivate `excelrs-core` + `excelrs-napi` workspace split. This is well within the spec's stated threshold.

---

## 2. Correctness Gaps (P0–P2)

### 🔴 P0 — CellValue enum is not mappable via `#[napi]` alone

**Spec reference:** Lines 449–462 (Rust enum), lines 393–404 (TypeScript API), lines 31–32 (P2: "no hand-written JS wrappers")

**What the spec claims:**
```rust
#[napi]
pub enum CellValue {
    Null,
    Number(f64),
    String(String),
    Boolean(bool),
    Date(DateTime<Utc>),
    // ...
}
```
…will generate a JS discriminated union:
```typescript
type CellValue = { type: 'Number'; value: number } | { type: 'String'; value: string } | ...
```

**Reality:** napi-rs v2/v3 **only supports C-style enums** (no variant data) via `#[napi(string_enum)]`. A Rust enum like:
```rust
#[napi(string_enum)]
enum Kind { Duck, Dog, Cat }
```
…generates:
```typescript
const enum Kind { Duck = 'Duck', Dog = 'Dog', Cat = 'Cat' }
```

**There is no `#[napi]` mapping for Rust enums with tuple/struct variants to JS discriminated unions.** The napi-rs documentation (concepts/enum) explicitly states: *"NAPI-RS doesn't support generating Rust enum impl into JavaScript."* Only string enum mappings are supported.

**Resolution options (must pick one before implementation):**

| Option | Pros | Cons |
|--------|------|------|
| **A: `#[napi(object)]` flat struct** — a single struct with optional fields for every variant | Pure `#[napi]`, no JS glue | Ugly API: `{ type: 'Number', numberValue: 42, stringValue: null, ... }`; wastes memory; breaks drop-in compatibility |
| **B: Hand-written JS wrapper** — Rust exposes internal representation + JS thin wrapper reconstructs discriminated union | Clean external API, matches exceljs | Violates P2 ("no hand-written JS"); requires maintaining JS glue |
| **C: Rust-side `JsObject` construction** — use `napi::Env` to manually build JS objects for each variant | Pure Rust, no JS files | Complex, error-prone, hard to maintain; still not `#[napi]`-derived |
| **D: Enum + separate value accessor** — `CellValueType` string enum + typed accessor methods (`cell.getNumberValue()`, `cell.getStringValue()`) | Pure `#[napi]`, no JS glue | Breaks drop-in compatibility with exceljs's `cell.value` being the tagged union |

**Recommendation:** Option B (thin JS wrapper). The hand-written code is ~80 lines, lives in one file, and is mechanically generated (not ongoing maintenance burden). The P2 principle can be relaxed from "no hand-written JS" to "no business logic in JS." Document this as a deliberate exception in the spec. Option D is the cleanest pure-Rust alternative but breaks the exceljs API contract.

### 🔴 P0 — Overloaded `getCell()` cannot be expressed in napi-rs

**Spec reference:** Lines 283–284

```typescript
getCell(address: string): Cell;
getCell(row: number, col: number): Cell;
```

napi-rs does not support method overloading. Rust also does not support method overloading. A single `#[napi]` method must have a single signature. Two methods with `#[napi(js_name = "getCell")]` would conflict at compile time.

**Resolution:** Use a single Rust method that takes a `JsUnknown` and branches internally:

```rust
#[napi]
pub fn get_cell(&self, a: JsUnknown, b: Option<u32>) -> Result<&Cell> {
    // If b is Some: (row, col) path
    // If b is None: a is either string (address) or number (row) — infer from type
}
```

This is workable but the spec should explicitly document: (1) the single-method approach, (2) the type-branching logic, (3) that `getCell(1)` with a single number arg is ambiguous (does it mean "get cell at row 1, col ?" or "get cell A1"?) — exceljs resolves this by treating single-number as `getRow()` call, which is the correct behavior.

### 🟡 P1 — OOXML inline strings not documented

**Spec reference:** Lines 164–167 (reader edge cases), line 185 (writer: "Handle shared strings")

OOXML supports two string representations:
1. **Shared strings**: `<c r="A1" t="s"><v>0</v></c>` — `v` is an index into `xl/sharedStrings.xml` (most common)
2. **Inline strings**: `<c r="A1" t="inlineStr"><is><t>text</t></is></c>` — string stored directly in the cell (rare, but valid)

The spec only mentions "shared strings" (line 185) and assumes calamine "resolves these" (line 166). calamine *does* handle both internally, but the **write path** (section 4.3) only describes shared string writing. If a user writes a cell value and excelrs always uses shared strings, round-trip with inline-string spreadsheets may produce structurally different (but semantically equivalent) files. This is fine for v0.1 but should be documented as a known limitation.

**Also:** Rich text inline strings (`<is><r><rPr>...</rPr><t>text</t></r></is>`) are a variant of inline strings that calamine may not fully parse into rich text components. The spec says calamine handles these — verify with test fixtures.

### 🟡 P1 — numFmt ID date detection is an unspecified heuristic

**Spec reference:** Line 167: "Dates stored as serial numbers with a number format flag → map to `CellValue::Date`"

Excel stores dates as numeric serial values with a **number format ID** (numFmtId) that indicates date formatting. The mapping is not trivial:

- Built-in date format IDs: 14–22, 27–36, 45–47, 50–81 (varying by locale)
- Custom date formats have IDs ≥ 164 and must be resolved from the styles table
- The 1900 vs 1904 date system affects serial number interpretation
- The Lotus 1-2-3 leap year bug (Feb 29, 1900 exists in Excel's 1900 system but not in reality)

The spec doesn't address:
1. Which numFmt IDs are treated as dates
2. Whether custom format strings are parsed for date indicators
3. How the 1900/1904 system is detected from workbook metadata
4. The leap-year-bug adjustment (serial day 60 = Feb 29, 1900, which is a phantom date)

calamine handles this internally (via `ExcelDateTime`), but the spec should reference calamine's date detection as authoritative for v0.1 and document that manual date coercion (writing a number then marking it as Date) will use a simplified heuristic.

### 🟡 P1 — Worksheet dimensions missing from write path

**Spec reference:** Lines 182–189 (write algorithm steps)

The OOXML spec requires (or strongly expects) a `<dimension ref="A1:C10"/>` element in `sheet.xml` that declares the used cell range. The spec's write algorithm lists 8 steps but doesn't mention writing dimensions. Excel uses this for scroll bounds and print area defaults. Omitting it produces files that open but may have incorrect scroll behavior.

**Fix:** Add step 4.5: "Compute `dimension` ref from min/max populated row and column, write `<dimension ref="..."/>` in sheet.xml."

### 🟡 P1 — Shared formula support not addressed

**Spec reference:** Line 186: "Write formula strings (preserved, not evaluated) as `<f>SUM(A1:A10)</f>`"

OOXML has two formula storage modes:
1. **Regular formula**: `<f>SUM(A1:A10)</f>` per cell
2. **Shared formula**: `<f t="shared" si="0">SUM(A1:A10)</f>` in first cell, `<f t="shared" si="0"/>` in subsequent cells

Shared formulas are common in real spreadsheets because they save space. The spec only describes regular formulas. If calamine reads a shared formula and excelrs writes it as a regular formula (or vice versa), round-trip fidelity is broken.

calamine exposes formulas via a **separate API** (`worksheet_formula()`), not via the regular cell data. The reader spec (lines 156–161) only calls `open_workbook_from_rs` and iterates cell data — it will **miss all formulas entirely**.

**Fix:** Add explicit formula reading step to section 4.2: "Call `worksheet_formula()` for each sheet, merge formula strings into corresponding cells by address." Acknowledge that shared formulas will be expanded to regular formulas on write in v0.1.

### 🟢 P2 — Missing OOXML edge cases (collective)

| Edge case | Where | Impact |
|-----------|-------|--------|
| Empty workbook (0 sheets) | Reader | Should return `Workbook { worksheets: vec![] }` — probably fine |
| Sheet with 0 rows, 0 cols | Reader | Spec says "valid Worksheet with row_count = 0" (line 165) ✅ |
| Cells with no `r` attribute | Reader | Position must be inferred from previous cell + gaps. calamine handles this ✅ |
| Cells with no `s` (style) attribute | Reader | Default style 0. Not referenced in spec |
| Cells with `t="e"` (error type) | Reader | calamine maps to `Data::Error`. Spec mentions `CellValue::Error` ✅ |
| Cells with `t="str"` (formula string result) | Reader | Legacy formula cache. calamine likely handles as string |
| `[Content_Types].xml` with overrides | Writer | Spec mentions (line 179) but doesn't detail override generation |
| Relationships ID uniqueness | Writer | Spec mentions (line 180, 187) but doesn't specify ID generation strategy |
| UTF-8 in sheet names | Both | Sheet names can contain Unicode. Not mentioned |
| Sheet names longer than 31 chars | Writer | Excel limit. Should truncate or error |
| Duplicate sheet names | Writer | Should error or auto-rename (exceljs auto-appends numbers) |

---

## 3. napi-rs Feasibility Audit (P0–P2)

### 🔴 P0 — Cells/Rows/Worksheets as `#[napi]` classes have reference-semantics problems

**Spec reference:** Lines 468–504 (Rust structs with `#[napi]`), lines 272–310 (TypeScript API)

The spec models `Worksheet.rows` as `BTreeMap<u32, Row>` (line 501) and `Row.cells` as `HashMap<u32, Cell>` (line 487). Both are `#[napi]` structs. This creates a **reference identity** problem:

```typescript
const cell1 = ws.getCell('A1');
const cell2 = ws.getCell('A1');
cell1.value = 42;
console.log(cell2.value); // ???
```

In the spec's Rust model, `getCell('A1')` returns a `Cell` by cloning from the HashMap. In JavaScript, `cell1` and `cell2` are **different JS objects** (because napi-rs structs are passed by value/clone). Mutating `cell1` doesn't affect the worksheet's internal state. This is **not** how exceljs works — exceljs cells are live references to the worksheet's internal cell map.

The napi-rs object docs (concepts/object) explicitly warn: *"The JavaScript Object passed in or returned from Rust is cloned. This means any mutation on JavaScript Object will not affect the original Rust struct."*

**Resolution options:**
- **A: Do not expose Cell/Row as standalone `#[napi]` classes.** Instead, expose methods on Worksheet/Workbook that read/write directly into the Rust state (e.g., `ws.setCellValue('A1', 42)`, `ws.getCellValue('A1')`). This deviates from exceljs's API but is correct.
- **B: Use interior mutability.** Wrap internal state in `Arc<Mutex<>>` or `RefCell` and have Cell hold a reference back to its parent Worksheet. Complex, may deadlock.
- **C: Accept the clone semantics.** Document that `cell.value = 42` only mutates the local JS object; users must call `ws.setCell('A1', cell)` to persist. Breaks drop-in compatibility.

**Recommendation:** Option A for v0.1. It's the simplest correct approach. Exceljs's chainable API (`ws.getCell('A1').value = 42`) can be approximated in v0.2 with interior mutability, but getting the fundamental read/write loop correct first matters more.

### 🟡 P1 — `#[napi]` structs with `pub` fields create JS constructor args, not mutable properties

**Spec reference:** Lines 468–478 (Cell struct with `pub` fields)

```rust
#[napi]
pub struct Cell {
    pub address: String,
    pub row: u32,
    pub col: u32,
    pub value: CellValue,
    // ...
}
```

napi-rs with `#[napi]` on a struct and `pub` fields generates a constructor that takes all fields as arguments: `new Cell(address, row, col, value, ...)`. It does **not** generate getters/setters for individual fields — you need `#[napi(getter)]` and `#[napi(setter)]` on explicit methods in an `impl` block.

The spec's TypeScript API shows property access (`cell.value`, `row.hidden`, `col.width`) but the Rust types don't have `#[napi(getter)]`/`#[napi(setter)]` annotations. This is a documentation gap — the spec correctly identifies that `#[napi]` generates JS classes (line 31), but doesn't detail the getter/setter contract.

**Fix:** Add getter/setter annotations to the Rust model section. Example:

```rust
#[napi]
impl Cell {
    #[napi(getter)]
    pub fn get_value(&self) -> CellValue { self.value.clone() }
    #[napi(setter)]
    pub fn set_value(&mut self, val: CellValue) { self.value = val; }
}
```

This is well-understood napi-rs usage but must be reflected in the spec's Rust type definitions.

### 🟡 P1 — `cell.value = 42` auto-wrapping to `CellValue::Number(42)` is impossible without JS glue

**Spec reference:** Line 829: "excelrs auto-wraps to `CellValue::Number(42)`"

In exceljs, the cell value setter does type inference:
```javascript
cell.value = 42;       // becomes { type: 'Number', value: 42 }
cell.value = 'hello';  // becomes { type: 'String', value: 'hello' }
cell.value = new Date(); // becomes { type: 'Date', value: ... }
```

With a `#[napi(setter)]` on a Rust method, the signature is fixed:
```rust
#[napi(setter)]
pub fn set_value(&mut self, val: CellValue) { ... }
```

This only accepts a `CellValue` object — `cell.value = 42` would be a type error. To accept plain values, you'd need either:
- A JS wrapper that does the type inference before calling the Rust setter
- Multiple setter overloads (not supported in napi-rs)
- A setter that takes `JsUnknown` and branches internally (complex)

**Recommendation:** Explicitly acknowledge this limitation. Options:
1. Accept `CellValue` objects only in v0.1: `cell.value = { type: 'Number', value: 42 }`
2. Add a thin JS wrapper (~30 lines) for the auto-wrapping convenience
3. Provide `cell.setNumberValue(42)`, `cell.setStringValue('hello')` as explicit setters

---

## 4. Missing Design Decisions (P1–P2)

### 🟡 P1 — Shared string deduplication strategy unspecified

**Spec reference:** Line 185: "deduplicate strings, write `xl/sharedStrings.xml`, reference by index"

The word "deduplicate" carries significant design weight. Unspecified:
- **Equality:** Case-sensitive? Unicode normalization (NFC/NFD)?
- **Data structure:** `HashMap<String, u32>` (key = string, value = index)?
- **Whitespace:** Does "hello" === "hello "? Excel preserves trailing whitespace in some cases.
- **Performance:** For 100K strings, a HashMap is fine. For 1M strings, may need interning.

**Recommendation:** Case-sensitive, exact byte equality, `HashMap<String, u32>`. Document as the spec. This matches exceljs behavior.

### 🟡 P1 — Cell reference validation boundary unspecified

**Spec reference:** Lines 231–235 (types.rs: `parse_address`, `address_to_string`)

Excel's cell reference system has well-defined limits:
- Columns: A through XFD (1–16384)
- Rows: 1 through 1,048,576

The spec defines `parse_address("A1") -> (col: u32, row: u32)` but doesn't specify:
- What happens on `parse_address("ZZZZ9999999")`? Overflow? Error? Truncation?
- What happens on `address_to_string(99999, 9999999)`? Return garbage or error?
- Should `getCell('Sheet1!A1')` with a sheet reference work? (exceljs: no)

**Recommendation:** Return `Result` (not panic) with `ExcelrsError::InvalidAddress` for out-of-bounds or malformed addresses. Add proptest: "for any valid col 1..=16384 and row 1..=1048576, `parse_address(address_to_string(col, row)) == (col, row)`."

### 🟡 P1 — Row creation semantics (eager vs sparse)

**Spec reference:** Line 142: "A Worksheet can have sparse rows." Line 287: "`getRow(rowNumber)` — Creates row if it doesn't exist."

The spec correctly identifies sparse storage (`BTreeMap<u32, Row>`, line 501). But `getRow(1000)` on an empty worksheet — does it:
- Create row 1000 only (sparse — correct)
- Create rows 1–1000 (eager — wrong, wastes memory)

The spec says "Creates row if it doesn't exist" which implies the correct sparse behavior. But it should also specify: does `getRow(1000)` affect `rowCount`? Answer: yes, `rowCount` should be max(1000, previous_rowCount). This is subtle — if row 1000 is deleted later, `rowCount` may need recalculation.

**Recommendation:** Explicitly state: "`getRow(n)` inserts a single row at index `n` into the `BTreeMap`. Rows 1..n-1 are NOT materialized. `rowCount` is `max(existing_rowCount, n)`. `removeRow(n)` removes row `n` but does NOT recalculate `rowCount` (lazy — recalculate on write)."

### 🟢 P2 — `CellValue::Merge` semantics undefined

**Spec reference:** Lines 400, 403, 461 (Merge variant exists but no semantics defined)

The `Merge` variant appears in the enum but the spec never says what it means, when it's set, or how it relates to merged-cell ranges. In exceljs, `Merge` on a cell means "this cell is merged into another (master) cell, its value comes from the master." The spec defers merged cell support to v0.2 (line 749), so why is `Merge` in the v0.1 CellValue enum?

**Recommendation:** Remove `Merge` and `RichText` from v0.1 `CellValue`. They are dead variants that cannot be read (calamine doesn't produce them) and cannot be written (no merged cell or rich text support in v0.1). Reintroduce in v0.2 when the features are implemented. Keeping dead variants wastes API surface and creates confusing "this exists but does nothing" scenarios.

### 🟢 P2 — Error recovery vs "fail loudly" tension

**Spec reference:** P4 (lines 38–40): "No silent failures. Every error is a typed `ExcelrsError` variant. Partial data is not returned."

This is a valid principle, but it has practical tension for spreadsheet parsing. A single malformed cell in a 100K-cell sheet should not prevent reading all other cells. The spec should differentiate:
- **Structural errors** (corrupt zip, missing required XML parts) → fail entirely
- **Content errors** (unrecognized numFmt, malformed single cell) → warn and skip, or report as warnings

Excel itself is extremely tolerant of malformed content. Strict failure on content errors means excelrs will reject files that Excel opens fine.

**Recommendation:** Add a "warnings" mechanism to the error model. Structural errors → `Err(ExcelrsError)`. Content errors → `Ok(workbook)` with a `warnings: Vec<String>` field on Workbook. This doesn't violate P4 — it's not "silent" if warnings are surfaced.

---

## 5. v0.1 Scope Realism (P2)

### 🟢 P2 — Scope is achievable with corrections

The v0.1 checklist (lines 730–744) has 15 items. After consolidation, the real work units are:

| Work unit | Est. effort | Notes |
|-----------|-------------|-------|
| Model types + getter/setter annotations | 3 days | Mostly mechanical from spec |
| CellValue FFI bridge | 3 days | The P0 re-architecture item |
| calamine reader + formula merge | 5 days | Formula requires separate API call |
| quick-xml writer + shared strings | 5 days | OOXML output is tedious but well-understood |
| Address parsing + date conversion | 2 days | Self-contained math |
| Error type + JS error mapping | 1 day | Mechanical |
| CI/CD pipeline | 2 days | Standard napi-rs CI template |
| Benchmarks | 2 days | Criterion + vitest bench |
| Exceljs round-trip test suite | 5 days | Port ~20 key fixtures, not 300 |
| **Total** | **~28 days** | Full-time solo dev, optimistic |

This is achievable for a motivated solo dev in 6–8 weeks. But "round-trip compatibility" (line 742) should be scoped to **20 key fixtures** (empty, single-sheet, multi-sheet, formulas, dates, numbers, strings, sparse, large) rather than porting the full 300-case exceljs test suite. The 300-case port is a v0.2 goal.

### 🟢 P2 — Items that can be deferred from v0.1

| Current v0.1 item | Defer to | Reason |
|-------------------|----------|--------|
| `CellStyle` populated on read (line 738) | v0.2 | Style reading requires mapping calamine style references to cell formats. Non-trivial, and v0.1 explicitly defers style CRUD |
| `RichText` and `Merge` variants (lines 459–461) | v0.2 | Dead code in v0.1 — no read or write support |
| `Hyperlink` variant (line 458) | v0.2 | calamine doesn't expose hyperlinks in cell data stream |
| `insta` snapshot tests (line 653) | v0.2 | Useful but not blocking. Plain assertions suffice for v0.1 |
| `proptest` for address parsing (line 652) | v0.1 | This is worth keeping — address parsing is self-contained and correctness-critical |
| `mockall` for reader/writer mocking (line 654) | v0.2 | Integration tests against real .xlsx fixtures are more valuable than mocks for v0.1 |

---

## 6. Top 3 Risks (P0)

### Risk 1: CellValue FFI misdesign forces re-architecture mid-implementation

**Probability:** Near-certain (the spec's `#[napi]` on `CellValue` enum with variant data will not compile).  
**Impact:** 2–3 weeks of rework to design and implement the FFI bridge correctly. Delays everything else because CellValue is the central type.  
**Mitigation:** Resolve the P0 FFI design decision (Option D or B from section 2) **before writing any Rust code**. Write a 20-line spike that compiles and passes a value across FFI.

### Risk 1b: Reference semantics mismatch between Rust (value) and JS (reference)

**Probability:** High.  
**Impact:** `ws.getCell('A1').value = 42` silently does nothing (mutates a clone, not the worksheet). Users report "excelrs is broken" as a data loss bug.  
**Mitigation:** As discussed in section 3, either: (a) do not expose Cell as a mutable JS object in v0.1, (b) use interior mutability with `Arc<Mutex<>>`, or (c) accept clone semantics and document prominently.

### Risk 2: Formula reading is silently broken because calamine stores formulas in a separate API

**Probability:** Certain if the spec's reader algorithm (lines 153–161) is implemented as written.  
**Impact:** Round-trip test fails — formulas present in input spreadsheet vanish in the output. This is a correctness P1 that blocks the v0.1 "round-trip compatibility" gate.  
**Mitigation:** Add explicit formula-reading step to the reader algorithm. Call `worksheet_formula()` for each sheet after reading cell data, merge by address. Test with a formula-containing fixture in the first week.

### Risk 3: Date handling edge cases cause silent data corruption

**Probability:** Medium-high (Excel date handling has well-documented edge cases).  
**Impact:** Dates shift by 1–4 years or display as serial numbers. Users report data corruption. Hard to debug because the serial number looks correct but the semantic date is wrong.  
**Mitigation:**
1. Explicitly test the 1900 leap-year bug (Feb 29, 1900 ⇄ serial 60)
2. Test the 1904 date system toggle (stored in workbook metadata)
3. Test fractional date serials (times: 0.5 = noon, 0.75 = 6pm)
4. Use calamine's `ExcelDateTime` conversion as the authoritative implementation — don't reimplement date math
5. Add a dedicated date round-trip test suite with edge cases from the Excel specs

---

## Appendix: Line-by-line issues

| Line(s) | Issue | Severity |
|---------|-------|----------|
| 31–32 | "No hand-written JS wrappers" contradicts P0 finding that CellValue needs JS glue | P0 |
| 156–161 | Reader algorithm doesn't call `worksheet_formula()` — formulas will be silently dropped | P1 |
| 164–167 | "Edge cases handled" lists shared strings and dates but misses inline strings, formula, hyperlink, rich text | P1 |
| 167 | "number format flag" for date detection is unspecified (which IDs? 1900/1904?) | P1 |
| 182–189 | Write algorithm missing `<dimension ref="..."/>` element | P1 |
| 185 | "deduplicate strings" — equality semantics unspecified | P1 |
| 186 | "Write formula strings" — doesn't mention shared formula expansion | P1 |
| 232–235 | Address parse functions don't specify error handling for out-of-bounds inputs | P1 |
| 283–284 | Overloaded `getCell` signatures cannot be expressed in napi-rs | P0 |
| 287 | `getRow` creation semantics (sparse vs eager) unspecified | P1 |
| 393–404 | Discriminated union TypeScript type is not derivable from `#[napi]` | P0 |
| 449–462 | `#[napi]` on `CellValue` enum with variant data will not compile | P0 |
| 459–461 | `RichText` and `Merge` variants are dead code in v0.1 | P2 |
| 470–474 | `pub` fields on Cell struct don't generate JS getter/setter properties | P1 |
| 493 | `HashMap<u32, Cell>` for row cells — no iteration order guarantee (use BTreeMap) | P2 |
| 535 | `CellStyle` struct uses `Option<String>` for colors — should be typed (e.g., ARGB) | P2 |
| 653 | `insta` snapshot tests — deferred to v0.2 | P2 |
| 654 | `mockall` for reader/writer mocking — deferred to v0.2 | P2 |
| 742 | "Round-trip compatibility with exceljs" — scope to 20 key fixtures | P2 |

---

*Review version: 1.0. Next step: spec revision addressing P0 items before implementation begins.*
