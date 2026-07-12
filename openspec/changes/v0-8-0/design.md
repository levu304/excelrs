## Context

excelrs lacks data validation read/write — `<dataValidations>` elements in
sheet XML are silently dropped on read, and there is no API to author them.
The model layer uses `#[napi(object)]` structs with `Arc<Mutex<>>` interior
mutability on Worksheet. Reader uses calamine for cell data + zip-based
parsing for per-sheet XML (styles, and now data validations). Writer emits
each sheet via `write_sheet_xml`.

## Goals / Non-Goals

**Goals:**

- `DataValidation` model type matching the OOXML `<dataValidation>` element
- Worksheet API: add/get/remove/list by `sqref` (range string)
- Writer emits `<dataValidations>` per sheet after hyperlinks
- Reader parses `<dataValidations>` from sheet XML via zip
- Supported: all 7 OOXML types, 8 operators, boolean flags, prompt/error messages
- exceljs-compatible interface shape

**Non-Goals:**

- No formula evaluation (formulae stored verbatim)
- No GUI prompt/error rendering
- No `dxfs`/conditional-formatting interaction
- No write-side `sqref` vs address difference (exceljs keys by single cell;
  our model keys by `sqref` range)

## Decisions

1. **`sqref` as the storage key** — exceljs uses single-cell addresses as keys
   in its internal model and derives `sqref` from range optimization at write
   time. excelrs stores the `sqref` directly as a field on `DataValidation`,
   and keys by it. This simplifies the model (no cell-level expansion) while
   preserving OOXML fidelity.

2. **Zip-based sheet XML parsing for reader** — calamine does not expose
   `<dataValidations>`. Following the existing styles pattern
   (`parse_styles_and_sheet_maps`), the reader opens the zip archive and
   parses each `xl/worksheets/sheet{N}.xml` for `<dataValidation>` elements
   via `quick_xml`. Same ponytail caveat about sheet-order numbering.

3. **`Arc<Mutex<Vec<DataValidation>>>`** — identical to the `merged_ranges`
   pattern. Ensures interior mutability across napi-rs clones.

4. **String type with `validate()`** — napi-rs doesn't support enum variants
   with data across FFI. `type` and `operator` are `String` fields with a
   `validate()` method checking against allowed values (matching `Fill.kind`
   in style.rs).

5. **Booleans emitted as `="1"` only when `true`** — OOXML defaults
   boolean attributes to 0/false. Omitting the attribute when false/None
   produces smaller, spec-compliant output.

6. **`formula1`/`formula2` model fields** — match the OOXML children
   `<formula1>` and `<formula2>` directly. exceljs uses a `formulae` array.
   Our writer emits `<formulaN>` elements; our reader parses them. The JS
   API uses `formula1`/`formula2` for string simplicity.

## Risks / Trade-offs

- **Sheet index → file number mapping** — reuses the same ponytail caveat as
  styles parsing: `sheet{N}.xml` numbering is assumed to match workbook sheet
  order. A fix would parse `xl/workbook.xml` rId→file mapping; defer until a
  real-world counterexample appears.
- **exceljs cross-check** — exceljs v4.4.0 omits operator="between" on write
  (default), and stores `formulae` as an array with numeric coercion. Our
  model uses strings. Both are valid OOXML; the cross-check test handles the
  mismatch.
